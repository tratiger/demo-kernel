use super::types::{VfsNode, VfsError};

pub trait FileOperations: Send + Sync {
    fn read(&self, node: &VfsNode, offset: usize, buf: &mut [u8]) -> Result<usize, VfsError>;
    fn readdir(&self, node: &VfsNode, buf: &mut [u8]) -> Result<usize, VfsError>;
}

pub trait FileSystem: Send + Sync {
    fn mount(&self) -> Result<(), VfsError>;
    fn open(&self, path: &str) -> Result<VfsNode, VfsError>;
}
