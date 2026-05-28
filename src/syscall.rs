use crate::println;
use crate::serial::SERIAL1;

#[unsafe(no_mangle)]
pub extern "C" fn syscall_dispatch(num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32 {
    match num {
        1 => {
            // sys_exit
            println!("User process exited with status: {}", arg1);
            loop {
                core::hint::spin_loop();
            }
        }
        3 => {
            // sys_read
            // arg1: fd
            // arg2: ptr
            // arg3: len
            let fd = arg1 as usize;
            let start = arg2;
            let len = arg3;

            // Strict bounds checking
            if start < 0x40000000 {
                return u32::MAX;
            }
            let end = start.checked_add(len);
            if end.is_none() {
                return u32::MAX;
            }
            let end = end.unwrap();
            if end > 0x40000000 + 4096 {
                return u32::MAX;
            }

            let mut thread_opt = crate::task::SCHEDULER.lock().current_thread.take();
            let mut read_bytes = u32::MAX;

            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if let Some((ref node, ref mut offset)) = thread.file_descriptors[fd] {
                        if let Some(read_func) = node.read {
                            let slice = unsafe {
                                core::slice::from_raw_parts_mut(start as *mut u8, len as usize)
                            };
                            if let Ok(bytes) = read_func(node, *offset, slice) {
                                *offset += bytes;
                                read_bytes = bytes as u32;
                            }
                        }
                    }
                }
                crate::task::SCHEDULER.lock().current_thread = Some(thread);
            }

            read_bytes
        }
        4 => {
            // sys_write
            // arg1: fd (1 for stdout)
            // arg2: ptr
            // arg3: len
            if arg1 != 1 {
                return u32::MAX; // Only stdout supported for now
            }

            // Strict bounds checking
            let start = arg2;
            let len = arg3;

            // 1. Lower Bound
            if start < 0x40000000 {
                return u32::MAX;
            }

            // 2. Overflow Check
            let end = start.checked_add(len);
            if end.is_none() {
                return u32::MAX;
            }
            let end = end.unwrap();

            // 3. Upper Bound
            if end > 0x40000000 + 4096 {
                return u32::MAX;
            }

            // Safe to read
            let slice = unsafe { core::slice::from_raw_parts(start as *const u8, len as usize) };

            // Try as UTF-8
            if let Ok(s) = core::str::from_utf8(slice) {
                crate::print!("{}", s);
            } else {
                // Fallback to byte writing
                let serial = SERIAL1.lock();
                for &b in slice {
                    serial.write_byte(b);
                }
            }

            len
        }
        5 => {
            // sys_open
            // arg1: name_ptr
            // arg2: name_len
            let start = arg1;
            let len = arg2;

            if start < 0x40000000 {
                return u32::MAX;
            }
            let end = start.checked_add(len);
            if end.is_none() {
                return u32::MAX;
            }
            let end = end.unwrap();
            if end > 0x40000000 + 4096 {
                return u32::MAX;
            }

            let slice = unsafe { core::slice::from_raw_parts(start as *const u8, len as usize) };
            if let Ok(path) = core::str::from_utf8(slice) {
                if let Ok(node) = crate::vfs::open(path) {
                    let mut fd_res = u32::MAX;
                    let mut thread_opt = crate::task::SCHEDULER.lock().current_thread.take();
                    if let Some(mut thread) = thread_opt {
                        // Find empty FD
                        let mut found_fd = None;
                        for (i, slot) in thread.file_descriptors.iter().enumerate() {
                            if slot.is_none() {
                                found_fd = Some(i);
                                break;
                            }
                        }

                        if let Some(fd) = found_fd {
                            thread.file_descriptors[fd] = Some((node, 0));
                            fd_res = fd as u32;
                        } else {
                            // Allocate new FD
                            let fd = thread.file_descriptors.len();
                            thread.file_descriptors.push(Some((node, 0)));
                            fd_res = fd as u32;
                        }
                        crate::task::SCHEDULER.lock().current_thread = Some(thread);
                    }
                    return fd_res;
                }
            }

            u32::MAX
        }
        _ => {
            println!("Unknown syscall: {}", num);
            u32::MAX
        }
    }
}
