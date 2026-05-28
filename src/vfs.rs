use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

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

pub type ReadFunction = fn(&VfsNode, usize, &mut [u8]) -> Result<usize, VfsError>;

#[derive(Clone)]
pub struct VfsNode {
    pub name: String,
    pub size: usize,
    pub file_type: FileType,
    pub data_ptr: u32,
    pub read: Option<ReadFunction>,
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

pub static VFS_ROOT: Mutex<Vec<VfsNode>> = Mutex::new(Vec::new());

pub fn mount(node: VfsNode) {
    VFS_ROOT.lock().push(node);
}

pub fn open(path: &str) -> Result<VfsNode, VfsError> {
    let root = VFS_ROOT.lock();
    for node in root.iter() {
        if node.name == path {
            return Ok(node.clone());
        }
    }
    Err(VfsError::FileNotFound)
}
