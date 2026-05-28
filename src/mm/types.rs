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
            head: Node {
                size: 0,
                next: None,
            },
        }
    }
}

pub struct KernelAllocator {
    pub inner: Mutex<DummyAllocator>,
}

pub struct MemoryFrameAllocator {
    pub bitmap: [u8; 131072],
}

impl MemoryFrameAllocator {
    pub const fn new() -> Self {
        Self {
            bitmap: [0; 131072],
        }
    }
}
