use crate::println;
use crate::drivers::char::serial::SERIAL1;

#[unsafe(no_mangle)]
pub extern "C" fn syscall_dispatch(num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32 {
    match num {
        3 => {
            let fd = arg1 as usize;
            let start = arg2;
            let len = arg3;
            if start < 0x40000000 {
                return u32::MAX;
            }
            if fd == 0 {
                let slice =
                    unsafe { core::slice::from_raw_parts_mut(start as *mut u8, len as usize) };
                let mut bytes_read = 0;
                while bytes_read < len as usize {
                    if let Some(c) = crate::drivers::char::keyboard::pop_char() {
                        slice[bytes_read] = c;
                        bytes_read += 1;
                        break;
                    } else {
                        // POLL SERIAL
                        unsafe {
                            let mut lsr = crate::arch::port::Port::new(0x3FD);
                            let status = lsr.read();
                            if (status & 1) != 0 {
                                let mut data = crate::arch::port::Port::new(0x3F8);
                                let mut c = data.read();
                                if c == b'\r' {
                                    c = b'\n';
                                } // Normalize enter
                                crate::drivers::char::keyboard::push_ascii(c);
                            } else {
                                crate::kernel::task::yield_task();
                            }
                        }
                    }
                }
                return bytes_read as u32;
            }
            let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
            let mut read_bytes = u32::MAX;
            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if let Some((ref node, ref mut offset)) = thread.file_descriptors[fd] {
                        if let Some(ref ops) = node.ops {
                            let slice = unsafe {
                                core::slice::from_raw_parts_mut(start as *mut u8, len as usize)
                            };
                            if let Ok(bytes) = ops.read(node, *offset, slice) {
                                *offset += bytes;
                                read_bytes = bytes as u32;
                            }
                        }
                    }
                }
                crate::kernel::task::SCHEDULER.lock().current_thread = Some(thread);
            }
            read_bytes
        }
4 => {
            let start = arg2;
            let len = arg3;
            if arg1 != 1 {
                return u32::MAX;
            }
            if start < 0x40000000 { return u32::MAX; }
            let end = start.checked_add(len);
            if end.is_none() { return u32::MAX; }
            let end = end.unwrap();
            let valid_data = end <= 0x40000000 + 40 * 4096;
            let valid_stack = start >= 0xA0000000 - 40 * 4096 && end <= 0xA0000000;
            if !valid_data && !valid_stack { return u32::MAX; }
            let slice = unsafe { core::slice::from_raw_parts(start as *const u8, len as usize) };
            if let Ok(s) = core::str::from_utf8(slice) {
                crate::print!("{}", s);
            } else {
                let serial = crate::drivers::char::serial::SERIAL1.lock();
                for &b in slice {
                    serial.write_byte(b);
                }
            }
            len
        }
5 => {
            let start = arg1;
            let len = arg2;
            if start < 0x40000000 { return u32::MAX; }
            let end = start.checked_add(len);
            if end.is_none() { return u32::MAX; }
            let end = end.unwrap();
            let valid_data = end <= 0x40000000 + 40 * 4096;
            let valid_stack = start >= 0xA0000000 - 40 * 4096 && end <= 0xA0000000;
            if !valid_data && !valid_stack { return u32::MAX; }
            let slice = unsafe { core::slice::from_raw_parts(start as *const u8, len as usize) };
            if let Ok(path) = core::str::from_utf8(slice) {
                let trimmed_path = path.trim_matches(char::from(0));
                if let Ok(node) = crate::fs::vfs_core::open(trimmed_path) {
                    let mut fd_res = u32::MAX;
                    let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
                    if let Some(mut thread) = thread_opt {
                        let mut found_fd = None;
                        for i in 3..thread.file_descriptors.len() {
                            if thread.file_descriptors[i].is_none() {
                                found_fd = Some(i);
                                break;
                            }
                        }
                        if let Some(fd) = found_fd {
                            thread.file_descriptors[fd] = Some((node, 0));
                            fd_res = fd as u32;
                        } else {
                            let fd = thread.file_descriptors.len();
                            thread.file_descriptors.push(Some((node, 0)));
                            fd_res = fd as u32;
                        }
                        crate::kernel::task::SCHEDULER.lock().current_thread = Some(thread);
                    }
                    return fd_res;
                } else {
                    crate::println!("[DEBUG] VFS Open failed for path: '{}'", trimmed_path);
                }
            } else {
                crate::println!("[DEBUG] Path UTF-8 conversion failed");
            }
            u32::MAX
        }
6 => {
            let fd = arg1 as usize;
            let start = arg2;
            let len = arg3;
            if start < 0x40000000 { return u32::MAX; }
            let end = start.checked_add(len);
            if end.is_none() { return u32::MAX; }
            let end = end.unwrap();
            let valid_data = end <= 0x40000000 + 40 * 4096;
            let valid_stack = start >= 0xA0000000 - 40 * 4096 && end <= 0xA0000000;
            if !valid_data && !valid_stack { return u32::MAX; }
            let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
            let mut read_bytes = u32::MAX;
            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if let Some(thread_fd) = thread.file_descriptors.get(fd) {
                        if let Some((node, _)) = thread_fd {
                            if let Some(ref ops) = node.ops {
                                let slice = unsafe {
                                    core::slice::from_raw_parts_mut(start as *mut u8, len as usize)
                                };
                                if let Ok(bytes) = ops.readdir(node, slice) {
                                    read_bytes = bytes as u32;
                                }
                            }
                        }
                    }
                }
                crate::kernel::task::SCHEDULER.lock().current_thread = Some(thread);
            }
            read_bytes
        }
        1 => {
            // sys_exit
            println!("User process exited with status: {}", arg1);
            loop {
                core::hint::spin_loop();
            }
        }

        _ => {
            println!("Unknown syscall: {}", num);
            u32::MAX
        }
    }
}
