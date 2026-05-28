use core::arch::asm;

pub struct Port {
    port: u16,
}

impl Port {
    pub const fn new(port: u16) -> Self {
        Port { port }
    }

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
