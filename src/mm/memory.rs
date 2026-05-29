use crate::mm::types::MemoryFrameAllocator;

pub static mut ALLOCATOR: MemoryFrameAllocator = MemoryFrameAllocator::new();

// Constants
pub const FRAME_SIZE: u32 = 4096;
pub const TOTAL_FRAMES: usize = 1048576; // 4GB / 4KB

pub unsafe fn init(initrd_info: Option<(u32, u32)>) {
    let frames_to_reserve = 2048;
    let bytes_to_reserve = frames_to_reserve / 8; // 256 bytes

    let allocator_ptr = core::ptr::addr_of_mut!(ALLOCATOR);

    unsafe {
        for i in 0..bytes_to_reserve {
            core::ptr::write_volatile(&mut (*allocator_ptr).bitmap[i], 0xFF);
        }
    }

    crate::println!("[Memory] Initialized physical frame allocator. First 8MB reserved.");

    if let Some((start, end)) = initrd_info {
        let start_frame = start / FRAME_SIZE;
        let end_frame = (end + FRAME_SIZE - 1) / FRAME_SIZE;

        for frame in start_frame..end_frame {
            let byte_index = (frame / 8) as usize;
            let bit_index = frame % 8;
            unsafe {
                let val = core::ptr::read_volatile(&(*allocator_ptr).bitmap[byte_index]);
                core::ptr::write_volatile(&mut (*allocator_ptr).bitmap[byte_index], val | (1 << bit_index));
            }
        }
        crate::println!(
            "[Memory] Reserved initrd frames: {} to {}",
            start_frame,
            end_frame - 1
        );
    }
}

pub unsafe fn allocate_frame() -> Option<u32> {
    let allocator_ptr = core::ptr::addr_of_mut!(ALLOCATOR);
    let len = unsafe { (*allocator_ptr).bitmap.len() };

    for i in 0..len {
        let val = unsafe { core::ptr::read_volatile(&(*allocator_ptr).bitmap[i]) };
        if val != 0xFF {
            for bit in 0..8 {
                if (val & (1 << bit)) == 0 {
                    unsafe { core::ptr::write_volatile(&mut (*allocator_ptr).bitmap[i], val | (1 << bit)) };
                    let frame_index = (i * 8) + bit;
                    return Some(frame_index as u32 * FRAME_SIZE);
                }
            }
        }
    }
    None // Out of memory
}

pub unsafe fn deallocate_frame(phys_addr: u32) {
    let frame_index = phys_addr / FRAME_SIZE;
    let byte_index = (frame_index / 8) as usize;
    let bit_index = frame_index % 8;

    let allocator_ptr = core::ptr::addr_of_mut!(ALLOCATOR);
    let len = unsafe { (*allocator_ptr).bitmap.len() };

    if byte_index < len {
        let val = unsafe { core::ptr::read_volatile(&(*allocator_ptr).bitmap[byte_index]) };
        unsafe {
            core::ptr::write_volatile(&mut (*allocator_ptr).bitmap[byte_index], val & !(1 << bit_index))
        };
    }
}
