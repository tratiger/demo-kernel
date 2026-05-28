use crate::vfs::{FileType, VfsError, VfsNode, mount};
use alloc::string::String;

#[repr(C, packed)]
struct TarHeader {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    chksum: [u8; 8],
    typeflag: u8,
    linkname: [u8; 100],
    magic: [u8; 6],
    version: [u8; 2],
    uname: [u8; 32],
    gname: [u8; 32],
    devmajor: [u8; 8],
    devminor: [u8; 8],
    prefix: [u8; 155],
    padding: [u8; 12],
}

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

        let magic = core::str::from_utf8(&header.magic).unwrap_or("");
        if !magic.starts_with("ustar") {
            crate::println!("[initrd] Invalid magic at {:#010X}", current_addr);
            break;
        }

        // Parse name
        let name_len = header.name.iter().position(|&c| c == 0).unwrap_or(100);
        let name = core::str::from_utf8(&header.name[..name_len]).unwrap_or("unknown");

        // Parse size
        let size = parse_octal(&header.size);

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
            read: Some(initrd_read),
            readdir: None,
        };
        mount(node);

        // Next file: 512 bytes for header + data, padded to 512-byte boundary
        let data_blocks = (size + 511) / 512;
        current_addr += 512 + (data_blocks as u32 * 512);
    }
}

pub fn initrd_readdir(_node: &VfsNode, buffer: &mut [u8]) -> Result<usize, VfsError> {
    let mut bytes_written = 0;
    let root = crate::vfs::VFS_ROOT.lock();

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

fn initrd_read(node: &VfsNode, offset: usize, buffer: &mut [u8]) -> Result<usize, VfsError> {
    if offset >= node.size {
        return Ok(0); // EOF
    }

    let remaining = node.size - offset;
    let read_size = core::cmp::min(remaining, buffer.len());

    let src = unsafe {
        core::slice::from_raw_parts((node.data_ptr as usize + offset) as *const u8, read_size)
    };

    buffer[..read_size].copy_from_slice(src);

    Ok(read_size)
}
