pub trait BlockDevice: Send + Sync {
    fn read_block(&self, block: usize, buf: &mut [u8]) -> Result<usize, ()>;
    fn write_block(&self, block: usize, buf: &[u8]) -> Result<usize, ()>;
}
