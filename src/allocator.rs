use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use spin::Mutex;

pub struct Node {
    pub size: usize,
    pub next: Option<&'static mut Node>,
}

pub struct DummyAllocator {
    pub head: Node,
}

impl DummyAllocator {
    pub const fn new() -> Self {
        Self {
            head: Node { size: 0, next: None },
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        let mut size = layout.size().max(core::mem::size_of::<Node>());
        let node_align = core::mem::align_of::<Node>();
        // Ensure size is a multiple of Node alignment so new nodes remain aligned
        if size % node_align != 0 {
            size += node_align - (size % node_align);
        }
        let align = layout.align();

        let mut current = &mut self.head;

        loop {
            let next_is_suitable = match current.next {
                Some(ref mut region) => {
                    let start = *region as *const Node as usize;
                    let end = start + region.size;
                    let alloc_start = (start + align - 1) & !(align - 1);
                    let alloc_end = alloc_start.checked_add(size).unwrap_or(usize::MAX);
                    alloc_end <= end
                }
                None => false,
            };

            if next_is_suitable {
                let node = current.next.take().unwrap();
                let start = node as *mut Node as usize;
                let end = start + node.size;
                let alloc_start = (start + align - 1) & !(align - 1);
                let alloc_end = alloc_start + size;
                let excess = end - alloc_end;

                let next = node.next.take();

                if excess >= core::mem::size_of::<Node>() {
                    let new_node_ptr = alloc_end as *mut Node;
                    unsafe {
                        (*new_node_ptr).size = excess;
                        (*new_node_ptr).next = next;
                        current.next = Some(&mut *new_node_ptr);
                    }
                } else {
                    current.next = next;
                }

                return alloc_start as *mut u8;
            }

            if current.next.is_none() {
                break;
            }
            current = current.next.as_mut().unwrap();
        }
        null_mut()
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(core::mem::size_of::<Node>());
        let new_node_ptr = ptr as *mut Node;

        unsafe {
            (*new_node_ptr).size = size;
            (*new_node_ptr).next = None;
        }

        // Find the correct insertion point to maintain address-sorted order
        let mut current = &mut self.head as *mut Node;
        let mut next = unsafe { (*current).next.as_mut().map(|n| *n as *mut Node) };

        while let Some(next_ptr) = next {
            if next_ptr > new_node_ptr {
                break;
            }
            current = next_ptr;
            next = unsafe { (*current).next.as_mut().map(|n| *n as *mut Node) };
        }

        // Insert new_node_ptr between current and next
        unsafe {
            (*new_node_ptr).next = if let Some(n) = next { Some(&mut *n) } else { None };
            (*current).next = Some(&mut *new_node_ptr);
        }

        // Coalesce: Look Ahead
        // Can we merge new_node_ptr with its next neighbor?
        unsafe {
            if let Some(ref mut next_node) = (*new_node_ptr).next {
                let next_node_ptr = *next_node as *mut Node;
                let new_node_end = (new_node_ptr as usize) + (*new_node_ptr).size;
                if new_node_end == (next_node_ptr as usize) {
                    (*new_node_ptr).size += (*next_node_ptr).size;
                    (*new_node_ptr).next = (*next_node_ptr).next.take();
                }
            }
        }

        // Coalesce: Look Behind
        // Can we merge current with new_node_ptr?
        // Note: self.head should never be merged with, as it's just a dummy head
        unsafe {
            let current_is_head = current == &mut self.head as *mut Node;
            if !current_is_head {
                let current_end = (current as usize) + (*current).size;
                if current_end == (new_node_ptr as usize) {
                    (*current).size += (*new_node_ptr).size;
                    (*current).next = (*new_node_ptr).next.take();
                }
            }
        }
    }
}

pub struct KernelAllocator {
    inner: Mutex<DummyAllocator>,
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner.lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.inner.lock().dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator {
    inner: Mutex::new(DummyAllocator::new()),
};

pub const HEAP_START: usize = 0x10000000;
pub const HEAP_SIZE: usize = 1 * 1024 * 1024;

pub unsafe fn init_heap() {
    let pages = HEAP_SIZE / crate::paging::PAGE_SIZE as usize;
    for i in 0..pages {
        let phys_frame = unsafe { crate::memory::allocate_frame().expect("No physical frames available for heap") };
        let virt_addr = HEAP_START + (i * crate::paging::PAGE_SIZE as usize);
        unsafe { crate::paging::map_page(virt_addr as u32, phys_frame, 3) }; // Present | R/W
    }

    // Initialize the dummy allocator list with the full mapped region
    let mut allocator = ALLOCATOR.inner.lock();
    let initial_node_ptr = HEAP_START as *mut Node;
    unsafe {
        (*initial_node_ptr).size = HEAP_SIZE;
        (*initial_node_ptr).next = None;
    }
    allocator.head.next = Some(unsafe { &mut *initial_node_ptr });
}
