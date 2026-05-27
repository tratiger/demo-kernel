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

pub struct SerialPort {
    data_port: Port,
    int_en_port: Port,
    fifo_ctrl_port: Port,
    line_ctrl_port: Port,
    modem_ctrl_port: Port,
    line_sts_port: Port,
}

impl SerialPort {
    pub const fn new(base_port: u16) -> Self {
        SerialPort {
            data_port: Port::new(base_port),
            int_en_port: Port::new(base_port + 1),
            fifo_ctrl_port: Port::new(base_port + 2),
            line_ctrl_port: Port::new(base_port + 3),
            modem_ctrl_port: Port::new(base_port + 4),
            line_sts_port: Port::new(base_port + 5),
        }
    }

    pub fn init(&self) {
        unsafe {
            // Disable all interrupts
            self.int_en_port.write(0x00);
            // Enable DLAB (set baud rate divisor)
            self.line_ctrl_port.write(0x80);
            // Set divisor to 1 (lo byte) 115200 baud
            self.data_port.write(0x01);
            //                  (hi byte)
            self.int_en_port.write(0x00);
            // 8 bits, no parity, one stop bit
            self.line_ctrl_port.write(0x03);
            // Enable FIFO, clear them, with 14-byte threshold
            self.fifo_ctrl_port.write(0xC7);
            // IRQs enabled, RTS/DSR set
            self.modem_ctrl_port.write(0x0B);
        }
    }

    fn is_transmit_empty(&self) -> bool {
        unsafe {
            // Bit 5 of the line status register tells us if the transmit buffer is empty
            self.line_sts_port.read() & 0x20 != 0
        }
    }

    pub fn write_byte(&self, byte: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }
        unsafe {
            self.data_port.write(byte);
        }
    }
}

impl core::fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            match byte {
                b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                }
                _ => self.write_byte(byte),
            }
        }
        Ok(())
    }
}

pub static SERIAL1: spin::Mutex<SerialPort> = spin::Mutex::new(SerialPort::new(0x3F8));

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).unwrap();
}
