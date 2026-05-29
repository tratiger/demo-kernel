use alloc::string::String;
use alloc::vec::Vec;
use alloc::sync::Arc;
use crate::kernel::sync::KernelMutex;
use crate::fs::traits::FileOperations;

use super::types::{VfsNode, VfsError, FileType};

pub static VFS_ROOT: KernelMutex<Option<Arc<KernelMutex<VfsNode>>>> = KernelMutex::new(None);
pub static VFS_OPS_TABLE: KernelMutex<Vec<Arc<dyn FileOperations>>> = KernelMutex::new(Vec::new());

pub fn init_root() {
    let mut root_opt = VFS_ROOT.lock();
    if root_opt.is_none() {
        *root_opt = Some(Arc::new(KernelMutex::new(VfsNode {
            name: String::from("/"),
            size: 0,
            file_type: FileType::Directory,
            data_ptr: 0,
            ops_index: 0,
            children: Vec::new(),
        })));
    }
}

pub fn mount(node: VfsNode) {
    let root_opt = VFS_ROOT.lock();
    if let Some(root_node) = root_opt.as_ref() {
        root_node.lock().children.push(Arc::new(KernelMutex::new(node)));
    }
}

pub fn open(path: &str) -> Result<VfsNode, VfsError> {
    if path == "/" || path == "." || path == "" {
        let root_opt = VFS_ROOT.lock();
        if let Some(root_node) = root_opt.as_ref() {
            return Ok(root_node.lock().clone());
        }
    }

    let root_opt = VFS_ROOT.lock();
    if let Some(root_node) = root_opt.as_ref() {
        // Very basic path resolution, simply searching children of root for now.
        // For a full tree, we'd split path by '/' and traverse.
        let path = path.trim_start_matches('/');
        for child in root_node.lock().children.iter() {
            let locked_child = child.lock();
            if locked_child.name == path {
                return Ok(locked_child.clone());
            }
        }
    }

    Err(VfsError::FileNotFound)
}

pub fn register_ops(ops: Arc<dyn FileOperations>) -> usize {
    let mut table = VFS_OPS_TABLE.lock();
    table.push(ops);
    table.len() - 1
}

pub fn get_ops(index: usize) -> Option<Arc<dyn FileOperations>> {
    let table = VFS_OPS_TABLE.lock();
    table.get(index).cloned()
}
