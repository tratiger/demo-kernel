use alloc::string::String;
use alloc::sync::Arc;
use crate::fs::traits::FileOperations;
use crate::fs::types::{VfsError, VfsNode, FileType};

pub struct InitrdOps;

impl FileOperations for InitrdOps {
    fn read(&self, node: &VfsNode, offset: usize, buf: &mut [u8]) -> Result<usize, VfsError> {
        if node.file_type != FileType::File {
            return Err(VfsError::IsADirectory);
        }
        if offset >= node.size {
            return Ok(0); // EOF
        }

        let bytes_to_read = core::cmp::min(buf.len(), node.size - offset);
        let src_ptr = (node.data_ptr as usize + offset) as *const u8;

        unsafe {
            core::ptr::copy_nonoverlapping(src_ptr, buf.as_mut_ptr(), bytes_to_read);
        }

        Ok(bytes_to_read)
    }

    fn write(&self, _node: &VfsNode, _offset: usize, _buf: &[u8]) -> Result<usize, VfsError> {
        Err(VfsError::PermissionDenied)
    }

    fn readdir(&self, _node: &VfsNode, buffer: &mut [u8]) -> Result<usize, VfsError> {
        let mut bytes_written = 0;
        let root = crate::fs::vfs_core::VFS_ROOT.lock();

        for file_node in root.iter() {
            let name_bytes = file_node.name.as_bytes();
            let len = name_bytes.len();

            if bytes_written + len + 1 > buffer.len() {
                break;
            }

            buffer[bytes_written..bytes_written + len].copy_from_slice(name_bytes);
            buffer[bytes_written + len] = 0;
            bytes_written += len + 1;
        }
        Ok(bytes_written)
    }
}
