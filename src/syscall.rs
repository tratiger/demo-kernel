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
        _ => {
            println!("Unknown syscall: {}", num);
            u32::MAX
        }
    }
}
