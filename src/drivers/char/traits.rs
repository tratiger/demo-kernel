pub trait CharDevice: Send + Sync {
    fn read(&self) -> Option<u8>;
    fn write(&self, byte: u8);
}
