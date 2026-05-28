#[repr(C, packed)]
pub struct MultibootInfo {
    pub flags: u32,
    pub mem_lower: u32,
    pub mem_upper: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub mods_count: u32,
    pub mods_addr: u32,
    pub syms: [u32; 4],
    pub mmap_length: u32,
    pub mmap_addr: u32,
}

#[repr(C, packed)]
pub struct MemoryMapEntry {
    pub size: u32,
    pub base_addr_low: u32,
    pub base_addr_high: u32,
    pub length_low: u32,
    pub length_high: u32,
    pub entry_type: u32,
}

#[repr(C, packed)]
pub struct MultibootModule {
    pub mod_start: u32,
    pub mod_end: u32,
    pub string: u32,
    pub reserved: u32,
}

pub fn parse(magic: u32, mbi_ptr: u32) -> Option<(u32, u32)> {
    if magic != 0x2BADB002 {
        crate::println!("[Multiboot] Invalid magic number: {:#X}", magic);
        return None;
    }

    let mbi = unsafe { &*(mbi_ptr as *const MultibootInfo) };

    let mut initrd_info = None;

    // Bit 3 indicates mods_* fields are valid
    if mbi.flags & (1 << 3) != 0 {
        let mods_count = mbi.mods_count;
        let mods_addr = mbi.mods_addr;
        crate::println!("[Multiboot] Modules detected: {}", mods_count);

        if mods_count > 0 {
            let module = unsafe { &*(mods_addr as *const MultibootModule) };
            let start = module.mod_start;
            let end = module.mod_end;
            crate::println!(
                "[Multiboot] Module 0: Start={:#010X}, End={:#010X}",
                start,
                end
            );
            initrd_info = Some((start, end));
        }
    } else {
        crate::println!("[Multiboot] No modules provided.");
    }

    // Bit 6 indicates mmap_* fields are valid
    let flags = mbi.flags;
    if flags & (1 << 6) != 0 {
        let mmap_length = mbi.mmap_length;
        let mmap_addr = mbi.mmap_addr;
        crate::println!("[Multiboot] Mmap detected (Length: {:#X})", mmap_length);

        let mut current_addr = mmap_addr;
        let end_addr = mmap_addr + mmap_length;

        while current_addr < end_addr {
            let entry = unsafe { &*(current_addr as *const MemoryMapEntry) };
            let entry_type = entry.entry_type;
            let base_addr_low = entry.base_addr_low;
            let length_low = entry.length_low;
            let size = entry.size;

            let type_str = match entry_type {
                1 => "Available",
                _ => "Reserved",
            };

            crate::println!(
                "MMAP: Base={:#010X}, Length={:#010X}, Type={}",
                base_addr_low,
                length_low,
                type_str
            );

            // size field does not include the size field itself (which is 4 bytes)
            current_addr += size + 4;
        }
    } else {
        crate::println!("[Multiboot] No memory map provided.");
    }

    initrd_info
}
