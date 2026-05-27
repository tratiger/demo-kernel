use core::arch::asm;

#[repr(C, packed)]
pub struct GdtPointer {
    limit: u16,
    base: u32,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct GdtEntry(u64);

impl GdtEntry {
    pub const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        let mut entry: u64 = 0;

        // Limit (bits 0-15)
        entry |= (limit & 0xFFFF) as u64;

        // Base (bits 16-39)
        entry |= ((base & 0xFFFFFF) as u64) << 16;

        // Access byte (bits 40-47)
        entry |= (access as u64) << 40;

        // Limit (bits 48-51)
        entry |= (((limit >> 16) & 0x0F) as u64) << 48;

        // Flags (bits 52-55)
        entry |= ((flags & 0x0F) as u64) << 52;

        // Base (bits 56-63)
        entry |= (((base >> 24) & 0xFF) as u64) << 56;

        Self(entry)
    }
}

#[repr(C, packed)]
pub struct TaskStateSegment {
    link: u16, _res0: u16,
    pub esp0: u32,
    pub ss0: u16, _res1: u16,
    esp1: u32,
    ss1: u16, _res2: u16,
    esp2: u32,
    ss2: u16, _res3: u16,
    cr3: u32,
    eip: u32,
    eflags: u32,
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    es: u16, _res4: u16,
    cs: u16, _res5: u16,
    ss: u16, _res6: u16,
    ds: u16, _res7: u16,
    fs: u16, _res8: u16,
    gs: u16, _res9: u16,
    ldtr: u16, _res10: u16,
    iopb_offset: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            link: 0, _res0: 0,
            esp0: 0,
            ss0: 0, _res1: 0,
            esp1: 0,
            ss1: 0, _res2: 0,
            esp2: 0,
            ss2: 0, _res3: 0,
            cr3: 0,
            eip: 0,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            es: 0, _res4: 0,
            cs: 0, _res5: 0,
            ss: 0, _res6: 0,
            ds: 0, _res7: 0,
            fs: 0, _res8: 0,
            gs: 0, _res9: 0,
            ldtr: 0, _res10: 0,
            iopb_offset: core::mem::size_of::<Self>() as u16, // No IOPB by default
        }
    }
}

pub static mut GDT: Gdt = Gdt::new();
pub static mut TSS: TaskStateSegment = TaskStateSegment::new();

unsafe extern "C" {
    safe static stack_top: u8;
}

pub fn init() {
    unsafe {
        // Set up TSS
        TSS.esp0 = &stack_top as *const u8 as u32;
        TSS.ss0 = 0x10; // Kernel Data Segment selector

        // Update GDT entry for TSS (0x28)
        let tss_base = core::ptr::addr_of!(TSS) as u32;
        let tss_limit = core::mem::size_of::<TaskStateSegment>() as u32 - 1;
        // Access byte: 0x89 (Present, Ring 0, 32-bit TSS, Available)
        // Flags: 0x0 (Byte granularity)
        GDT.entries[5] = GdtEntry::new(tss_base, tss_limit, 0x89, 0x0);

        // Load GDT
        let gdt_ptr = GdtPointer {
            limit: (core::mem::size_of::<Gdt>() - 1) as u16,
            base: core::ptr::addr_of!(GDT) as u32,
        };
        asm!("lgdt [{}]", in(reg) &gdt_ptr, options(readonly, nostack, preserves_flags));

        // Load TSS
        asm!("ltr ax", in("ax") 0x28u16, options(nomem, nostack, preserves_flags));

        // Reload data segment registers
        asm!(
            "mov ds, ax",
            "mov es, ax",
            "mov fs, ax",
            "mov gs, ax",
            "mov ss, ax",
            in("ax") 0x10u16,
            options(nomem, nostack, preserves_flags)
        );

        // Reload code segment register using far return trick
        asm!(
            "push 0x08",
            "call 2f",
            "2:",
            "add dword ptr [esp], 5",
            "retf",
            options(nomem, nostack, preserves_flags)
        );
    }
}

pub struct Gdt {
    entries: [GdtEntry; 6],
}

impl Gdt {
    pub const fn new() -> Self {
        Self {
            entries: [
                // 0x00: Null descriptor
                GdtEntry::new(0, 0, 0, 0),
                // 0x08: Kernel Code Segment (Ring 0, executable/readable, base 0, limit 4GB)
                GdtEntry::new(0, 0xFFFFF, 0x9A, 0xC),
                // 0x10: Kernel Data Segment (Ring 0, readable/writable, base 0, limit 4GB)
                GdtEntry::new(0, 0xFFFFF, 0x92, 0xC),
                // 0x18: User Code Segment (Ring 3, executable/readable, base 0, limit 4GB)
                GdtEntry::new(0, 0xFFFFF, 0xFA, 0xC),
                // 0x20: User Data Segment (Ring 3, readable/writable, base 0, limit 4GB)
                GdtEntry::new(0, 0xFFFFF, 0xF2, 0xC),
                // 0x28: TSS descriptor (will be updated at runtime)
                GdtEntry::new(0, 0, 0, 0),
            ],
        }
    }
}
