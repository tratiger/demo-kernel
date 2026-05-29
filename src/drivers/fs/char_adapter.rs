use crate::fs::traits::FileOperations;
use crate::fs::types::{VfsError, VfsNode};
use crate::drivers::traits::CharDevice;
use alloc::sync::Arc;

pub struct CharDeviceAdapter {
    pub device: Arc<dyn CharDevice>,
}

impl FileOperations for CharDeviceAdapter {
    fn read(&self, _node: &VfsNode, _offset: usize, buf: &mut [u8]) -> Result<usize, VfsError> {
        let mut bytes_read = 0;
        while bytes_read < buf.len() {
            if let Some(b) = self.device.read() {
                buf[bytes_read] = b;
                bytes_read += 1;
                // For stdin interactiveness, we usually return on first read if it's a newline,
                // but let's just break immediately so the shell can process byte-by-byte.
                break;
            } else {
                crate::kernel::task::yield_task();
            }
        }
        Ok(bytes_read)
    }

    fn write(&self, _node: &VfsNode, _offset: usize, buf: &[u8]) -> Result<usize, VfsError> {
        for &b in buf {
            self.device.write(b);
        }
        Ok(buf.len())
    }

    fn readdir(&self, _node: &VfsNode, _buf: &mut [u8]) -> Result<usize, VfsError> {
        Err(VfsError::NotADirectory)
    }
}
