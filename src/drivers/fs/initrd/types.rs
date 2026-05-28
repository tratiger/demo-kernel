// types for initrd
#[repr(C)]
pub struct TarHeader {
    pub name: [u8; 100],
    pub mode: [u8; 8],
    pub uid: [u8; 8],
    pub gid: [u8; 8],
    pub size: [u8; 12],
    pub mtime: [u8; 12],
    pub chksum: [u8; 8],
    pub typeflag: u8,
}

impl TarHeader {
    pub fn size(&self) -> usize {
        let mut size = 0;
        for i in 0..11 {
            if self.size[i] >= b'0' && self.size[i] <= b'7' {
                size = size * 8 + (self.size[i] - b'0') as usize;
            }
        }
        size
    }

    pub fn name(&self) -> &str {
        let mut len = 0;
        while len < 100 && self.name[len] != 0 {
            len += 1;
        }
        core::str::from_utf8(&self.name[0..len]).unwrap_or("")
    }
}
