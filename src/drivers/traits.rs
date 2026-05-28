pub trait CharDevice: Send + Sync {
    fn read(&self) -> Option<u8>;
    fn write(&self, byte: u8);
}

pub trait BlockDevice: Send + Sync {
    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<usize, ()>;
    fn write_block(&self, block: usize, buf: &[u8]) -> Result<usize, ()>;
}
