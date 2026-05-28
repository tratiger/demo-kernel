use core::arch::asm;
use crate::arch::types::Port;

impl Port {
    pub unsafe fn write(&self, data: u8) {
        unsafe {
            asm!("out dx, al", in("dx") self.port, in("al") data, options(nomem, nostack, preserves_flags));
        }
    }

    pub unsafe fn read(&self) -> u8 {
        let mut data: u8;
        unsafe {
            asm!("in al, dx", out("al") data, in("dx") self.port, options(nomem, nostack, preserves_flags));
        }
        data
    }
}
