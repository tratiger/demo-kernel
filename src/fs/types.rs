use alloc::string::String;
use alloc::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone)]
pub enum VfsError {
    FileNotFound,
    IsADirectory,
    NotADirectory,
    PermissionDenied,
    InvalidOffset,
    IoError,
}

#[derive(Clone)]
pub struct VfsNode {
    pub name: String,
    pub size: usize,
    pub file_type: FileType,
    pub data_ptr: u32,
    pub ops_index: usize,
    pub children: alloc::vec::Vec<alloc::sync::Arc<crate::kernel::sync::KernelMutex<VfsNode>>>,
}

impl core::fmt::Debug for VfsNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VfsNode")
            .field("name", &self.name)
            .field("size", &self.size)
            .field("file_type", &self.file_type)
            .field("data_ptr", &self.data_ptr)
            .finish()
    }
}
