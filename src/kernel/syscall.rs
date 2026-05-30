use crate::println;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Syscall {
    Exit = 1,
    Read = 3,
    Write = 4,
    Open = 5,
    Readdir = 6,
    Close = 7,
}

impl Syscall {
    pub fn from_u32(num: u32) -> Option<Self> {
        match num {
            1 => Some(Self::Exit),
            3 => Some(Self::Read),
            4 => Some(Self::Write),
            5 => Some(Self::Open),
            6 => Some(Self::Readdir),
            7 => Some(Self::Close),
            _ => None,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn syscall_dispatch(num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32 {
    match Syscall::from_u32(num) {
        Some(Syscall::Read) => {
            let fd = arg1 as usize;
            let start = arg2;
            let len = arg3;
            if !crate::mm::is_user_memory(start, len) {
                return u32::MAX;
            }
            let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
            let mut read_bytes = u32::MAX;
            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if let Some((ref node, ref mut offset)) = thread.file_descriptors[fd] {
                        if let Some(ops) = crate::fs::vfs_core::get_ops(node.ops_index) {
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
        Some(Syscall::Write) => {
            let fd = arg1 as usize;
            let start = arg2;
            let len = arg3;
            if !crate::mm::is_user_memory(start, len) {
                return u32::MAX;
            }
            let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
            let mut write_bytes = u32::MAX;
            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if let Some((ref node, ref mut offset)) = thread.file_descriptors[fd] {
                        if let Some(ops) = crate::fs::vfs_core::get_ops(node.ops_index) {
                            let slice = unsafe { core::slice::from_raw_parts(start as *const u8, len as usize) };
                            if let Ok(bytes) = ops.write(node, *offset, slice) {
                                *offset += bytes;
                                write_bytes = bytes as u32;
                            }
                        }
                    }
                }
                crate::kernel::task::SCHEDULER.lock().current_thread = Some(thread);
            }
            write_bytes
        }
        Some(Syscall::Open) => {
            let start = arg1;
            let len = arg2;
            if !crate::mm::is_user_memory(start, len) {
                return u32::MAX;
            }
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
        Some(Syscall::Readdir) => {
            let fd = arg1 as usize;
            let start = arg2;
            let len = arg3;
            if !crate::mm::is_user_memory(start, len) {
                return u32::MAX;
            }
            let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
            let mut read_bytes = u32::MAX;
            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if let Some(thread_fd) = thread.file_descriptors.get(fd) {
                        if let Some((node, _)) = thread_fd {
                            if let Some(ops) = crate::fs::vfs_core::get_ops(node.ops_index) {
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
        Some(Syscall::Close) => {
            let fd = arg1 as usize;
            let mut thread_opt = crate::kernel::task::SCHEDULER.lock().current_thread.take();
            let mut result = u32::MAX;
            if let Some(mut thread) = thread_opt {
                if fd < thread.file_descriptors.len() {
                    if thread.file_descriptors[fd].is_some() {
                        thread.file_descriptors[fd] = None;
                        result = 0;
                    }
                }
                crate::kernel::task::SCHEDULER.lock().current_thread = Some(thread);
            }
            result
        }
        Some(Syscall::Exit) => {
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
