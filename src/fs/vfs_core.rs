use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use super::types::{VfsNode, VfsError, FileType};

pub static VFS_ROOT: Mutex<Vec<VfsNode>> = Mutex::new(Vec::new());

pub fn mount(node: VfsNode) {
    VFS_ROOT.lock().push(node);
}

pub fn open(path: &str) -> Result<VfsNode, VfsError> {
    if path == "/" || path == "." || path == "" {
        // Find root node instead of hardcoding initrd
        let root = VFS_ROOT.lock();
        for node in root.iter() {
            if node.name == "/" {
                return Ok(node.clone());
            }
        }
        // If not found, return a dummy root for readdir
        return Ok(VfsNode {
            name: String::from("/"),
            size: 0,
            file_type: FileType::Directory,
            data_ptr: 0,
            ops: Some(alloc::sync::Arc::new(crate::drivers::fs::initrd::impl_vfs::InitrdOps)),
        });
    }

    let root = VFS_ROOT.lock();
    for node in root.iter() {
        if node.name == path {
            return Ok(node.clone());
        }
    }
    Err(VfsError::FileNotFound)
}
