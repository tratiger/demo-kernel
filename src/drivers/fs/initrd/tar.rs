use crate::fs::types::{FileType, VfsError, VfsNode};

use alloc::string::String;
use alloc::sync::Arc;
use crate::drivers::fs::initrd::impl_vfs::InitrdOps;
use crate::drivers::fs::initrd::types::TarHeader;

// TarHeader is now in types.rs, so we don't define it here

fn parse_octal(bytes: &[u8]) -> usize {
    let mut res = 0;
    for &b in bytes {
        if b >= b'0' && b <= b'7' {
            res = res * 8 + (b - b'0') as usize;
        } else if b == 0 || b == b' ' {
            // Null or space terminates the number in tar
            if res != 0 {
                break;
            }
        } else {
            break;
        }
    }
    res
}

pub fn init(start_addr: u32, end_addr: u32) {
    crate::println!(
        "[initrd] Initializing from {:#010X} to {:#010X}",
        start_addr,
        end_addr
    );

    let mut current_addr = start_addr;

    while current_addr < end_addr {
        let header = unsafe { &*(current_addr as *const TarHeader) };

        // Check for end of archive (two empty 512-byte blocks)
        if header.name[0] == 0 {
            break;
        }

        // We simplified TarHeader in types.rs without magic. Let's just trust it for now or do a simple check.
        // For a demo kernel we can check size parsing to see if it is sensible.

        let name = header.name();
        let size = header.size();

        let file_type = if header.typeflag == b'5' {
            FileType::Directory
        } else {
            FileType::File
        };

        crate::println!(
            "[initrd] Found file: {} (Size: {} bytes, Type: {:?})",
            name,
            size,
            file_type
        );

        let data_ptr = current_addr + 512;

        let node = VfsNode {
            name: String::from(name),
            size,
            file_type,
            data_ptr,
            ops_index: 1,
            children: alloc::vec::Vec::new(),
        };
        crate::fs::vfs_core::mount(node);

        // Next file: 512 bytes for header + data, padded to 512-byte boundary
        let data_blocks = (size + 511) / 512;
        current_addr += 512 + (data_blocks as u32 * 512);
    }
}

// read and readdir functions moved to impl_vfs.rs
