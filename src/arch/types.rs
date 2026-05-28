#[repr(C, packed)]
pub struct GdtPointer {
    pub limit: u16,
    pub base: u32,
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct GdtEntry(pub u64);

impl GdtEntry {
    pub const fn new(base: u32, limit: u32, access: u8, flags: u8) -> Self {
        let mut entry: u64 = 0;
        entry |= (limit & 0xFFFF) as u64;
        entry |= ((base & 0xFFFFFF) as u64) << 16;
        entry |= (access as u64) << 40;
        entry |= (((limit >> 16) & 0x0F) as u64) << 48;
        entry |= ((flags & 0x0F) as u64) << 52;
        entry |= (((base >> 24) & 0xFF) as u64) << 56;
        Self(entry)
    }
}

pub struct Gdt {
    pub entries: [GdtEntry; 6],
}

impl Gdt {
    pub const fn new() -> Self {
        Self {
            entries: [
                GdtEntry::new(0, 0, 0, 0),
                GdtEntry::new(0, 0xFFFFF, 0x9A, 0xC),
                GdtEntry::new(0, 0xFFFFF, 0x92, 0xC),
                GdtEntry::new(0, 0xFFFFF, 0xFA, 0xC),
                GdtEntry::new(0, 0xFFFFF, 0xF2, 0xC),
                GdtEntry::new(0, 0, 0, 0),
            ],
        }
    }
}

#[repr(C, packed)]
pub struct TaskStateSegment {
    link: u16,
    _res0: u16,
    pub esp0: u32,
    pub ss0: u16,
    _res1: u16,
    esp1: u32,
    ss1: u16,
    _res2: u16,
    esp2: u32,
    ss2: u16,
    _res3: u16,
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
    es: u16,
    _res4: u16,
    cs: u16,
    _res5: u16,
    ss: u16,
    _res6: u16,
    ds: u16,
    _res7: u16,
    fs: u16,
    _res8: u16,
    gs: u16,
    _res9: u16,
    ldtr: u16,
    _res10: u16,
    iopb_offset: u16,
}

impl TaskStateSegment {
    pub const fn new() -> Self {
        Self {
            link: 0, _res0: 0, esp0: 0, ss0: 0, _res1: 0, esp1: 0, ss1: 0, _res2: 0, esp2: 0,
            ss2: 0, _res3: 0, cr3: 0, eip: 0, eflags: 0, eax: 0, ecx: 0, edx: 0, ebx: 0,
            esp: 0, ebp: 0, esi: 0, edi: 0, es: 0, _res4: 0, cs: 0, _res5: 0, ss: 0,
            _res6: 0, ds: 0, _res7: 0, fs: 0, _res8: 0, gs: 0, _res9: 0, ldtr: 0, _res10: 0,
            iopb_offset: core::mem::size_of::<Self>() as u16,
        }
    }
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    pub offset_low: u16,
    pub selector: u16,
    pub zero: u8,
    pub type_attr: u8,
    pub offset_high: u16,
}

impl IdtEntry {
    pub const fn new() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0,
            zero: 0,
            type_attr: 0,
            offset_high: 0,
        }
    }

    pub fn set_handler_fn(&mut self, handler: u32) {
        self.offset_low = handler as u16;
        self.offset_high = (handler >> 16) as u16;
        self.selector = 0x08; // Code segment from GDT
        self.type_attr = 0x8E; // Present, Ring 0, 32-bit Interrupt Gate
    }

    pub fn set_handler_fn_trap_user(&mut self, handler: u32) {
        self.offset_low = handler as u16;
        self.offset_high = (handler >> 16) as u16;
        self.selector = 0x08; // Code segment from GDT
        self.type_attr = 0xEF; // Present, Ring 3 (DPL=3), System=0, 32-bit Trap Gate
    }
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct InterruptDescriptorTable {
    pub entries: [IdtEntry; 256],
}

impl InterruptDescriptorTable {
    pub const fn new() -> Self {
        InterruptDescriptorTable {
            entries: [IdtEntry::new(); 256],
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IdtPointer {
    pub limit: u16,
    pub base: u32,
}

#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
}

#[repr(C, align(4096))]
pub struct PageDirectory {
    pub entries: [u32; 1024],
}

#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [u32; 1024],
}

impl PageDirectory {
    pub const fn new() -> Self {
        PageDirectory { entries: [0; 1024] }
    }
}

impl PageTable {
    pub const fn new() -> Self {
        PageTable { entries: [0; 1024] }
    }
}

#[derive(Clone, Copy)]
pub struct Port {
    pub port: u16,
}

impl Port {
    pub const fn new(port: u16) -> Port {
        Port { port }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct TaskContext {
    pub edi: u32,
    pub esi: u32,
    pub ebx: u32,
    pub ebp: u32,
    pub eip: u32,
}
