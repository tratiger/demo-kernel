use crate::drivers::traits::CharDevice;
use crate::drivers::char::serial::SERIAL1;
use crate::drivers::char::keyboard::Keyboard;

pub struct Stdio;

impl CharDevice for Stdio {
    fn read(&self) -> Option<u8> {
        // Try keyboard first
        if let Some(c) = Keyboard.read() {
            return Some(c);
        }
        // Then try serial
        SERIAL1.lock().read()
    }

    fn write(&self, byte: u8) {
        SERIAL1.lock().write(byte);
    }
}
