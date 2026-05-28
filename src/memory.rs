// 4GB of physical memory in 4KB frames is 1,048,576 frames.
// 1,048,576 frames / 8 bits per byte = 131,072 bytes (128 KB).
pub static mut BITMAP: [u8; 131072] = [0; 131072];

// Constants
pub const FRAME_SIZE: u32 = 4096;
pub const TOTAL_FRAMES: usize = 1048576; // 4GB / 4KB

pub unsafe fn init(initrd_info: Option<(u32, u32)>) {
    // The kernel and initial structures reside in the first 8MB of physical memory.
    // 8MB / 4KB = 2048 frames.
    // We mask the first 2048 frames as "used" so the allocator doesn't hand them out.
    let frames_to_reserve = 2048;
    let bytes_to_reserve = frames_to_reserve / 8; // 256 bytes

    unsafe {
        for i in 0..bytes_to_reserve {
            core::ptr::write_volatile(&mut BITMAP[i], 0xFF); // All 8 frames in this byte are marked as used (1)
        }
    }

    crate::println!("[Memory] Initialized physical frame allocator. First 8MB reserved.");

    if let Some((start, end)) = initrd_info {
        let start_frame = start / FRAME_SIZE;
        // round up to the next frame
        let end_frame = (end + FRAME_SIZE - 1) / FRAME_SIZE;

        for frame in start_frame..end_frame {
            let byte_index = (frame / 8) as usize;
            let bit_index = frame % 8;
            unsafe {
                let val = core::ptr::read_volatile(&BITMAP[byte_index]);
                core::ptr::write_volatile(&mut BITMAP[byte_index], val | (1 << bit_index));
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
    let bitmap_ptr = core::ptr::addr_of_mut!(BITMAP);
    let len = unsafe { (*bitmap_ptr).len() };

    for i in 0..len {
        let val = unsafe { core::ptr::read_volatile(&(*bitmap_ptr)[i]) };
        if val != 0xFF {
            // There's at least one free frame in this byte
            for bit in 0..8 {
                if (val & (1 << bit)) == 0 {
                    // Frame is free! Mark as used.
                    unsafe { core::ptr::write_volatile(&mut (*bitmap_ptr)[i], val | (1 << bit)) };
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

    let bitmap_ptr = core::ptr::addr_of_mut!(BITMAP);
    let len = unsafe { (*bitmap_ptr).len() };

    if byte_index < len {
        let val = unsafe { core::ptr::read_volatile(&(*bitmap_ptr)[byte_index]) };
        unsafe {
            core::ptr::write_volatile(&mut (*bitmap_ptr)[byte_index], val & !(1 << bit_index))
        };
    }
}
