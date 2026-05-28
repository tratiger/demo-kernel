#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    zero: u8,
    type_attr: u8,
    offset_high: u16,
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
    entries: [IdtEntry; 256],
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
    limit: u16,
    base: u32,
}

impl IdtPointer {
    pub fn new(idt: &InterruptDescriptorTable) -> Self {
        IdtPointer {
            limit: (core::mem::size_of::<InterruptDescriptorTable>() - 1) as u16,
            base: idt as *const _ as u32,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub ip: u32,
    pub cs: u32,
    pub flags: u32,
}

pub extern "x86-interrupt" fn breakpoint_handler(_frame: InterruptStackFrame) {
    crate::println!("EXCEPTION: BREAKPOINT");
}

pub extern "x86-interrupt" fn double_fault_handler(
    _frame: InterruptStackFrame,
    error_code: u32,
) -> ! {
    crate::println!("EXCEPTION: DOUBLE FAULT\nError Code: {:#X}", error_code);
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
    }
}

pub extern "x86-interrupt" fn page_fault_handler(_frame: InterruptStackFrame, error_code: u32) {
    let mut cr2: u32;
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));
    }
    crate::println!(
        "PAGE FAULT at address: {:#X} (Error Code: {:#X})",
        cr2,
        error_code
    );
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
    }
}

#[unsafe(naked)]
pub unsafe extern "C" fn syscall_stub() {
    core::arch::naked_asm!(
        "push edx",
        "push ecx",
        "push ebx",
        "push eax",
        "call syscall_dispatch",
        "add esp, 16",
        "iretd",
    );
}

unsafe extern "C" {
    fn syscall_dispatch(num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32;
}

pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init_idt() {
    unsafe {
        IDT.entries[3].set_handler_fn(breakpoint_handler as *const () as u32);
        IDT.entries[8].set_handler_fn(double_fault_handler as *const () as u32);
        IDT.entries[14].set_handler_fn(page_fault_handler as *const () as u32);
        IDT.entries[PIC1_OFFSET as usize]
            .set_handler_fn(timer_interrupt_handler as *const () as u32);
        IDT.entries[PIC1_OFFSET as usize + 1]
            .set_handler_fn(keyboard_interrupt_handler as *const () as u32);

        IDT.entries[0x80].set_handler_fn_trap_user(syscall_stub as *const () as u32);

        let idt_ptr = IdtPointer {
            limit: (core::mem::size_of::<InterruptDescriptorTable>() - 1) as u16,
            base: core::ptr::addr_of!(IDT) as u32,
        };
        core::arch::asm!("lidt [{}]", in(reg) &idt_ptr, options(readonly, nostack, preserves_flags));
    }
}

use crate::arch::port::Port;

pub const PIC1_COMMAND: Port = Port::new(0x20);
pub const PIC1_DATA: Port = Port::new(0x21);
pub const PIC2_COMMAND: Port = Port::new(0xA0);
pub const PIC2_DATA: Port = Port::new(0xA1);

pub const PIC1_OFFSET: u8 = 32;
pub const PIC2_OFFSET: u8 = PIC1_OFFSET + 8;

pub fn init_pic() {
    unsafe {
        let _a1 = PIC1_DATA.read();
        let _a2 = PIC2_DATA.read();

        // ICW1: Init
        PIC1_COMMAND.write(0x11);
        PIC2_COMMAND.write(0x11);

        // ICW2: Vector offset
        PIC1_DATA.write(PIC1_OFFSET);
        PIC2_DATA.write(PIC2_OFFSET);

        // ICW3: Cascading
        PIC1_DATA.write(0x04); // PIC2 at IRQ2
        PIC2_DATA.write(0x02); // Cascade identity

        // ICW4: Mode (8086)
        PIC1_DATA.write(0x01);
        PIC2_DATA.write(0x01);

        // Unmask IRQ0 (Timer) and IRQ1 (Keyboard), mask others
        PIC1_DATA.write(0xFC);
        PIC2_DATA.write(0xFF);
    }
}

pub fn send_eoi(interrupt_id: u8) {
    unsafe {
        if interrupt_id >= PIC2_OFFSET {
            PIC2_COMMAND.write(0x20);
        }
        PIC1_COMMAND.write(0x20);
    }
}

pub static mut TIMER_TICKS: u64 = 0;

pub extern "x86-interrupt" fn timer_interrupt_handler(_frame: InterruptStackFrame) {
    unsafe {
        TIMER_TICKS += 1;
    }
    send_eoi(PIC1_OFFSET);
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_frame: InterruptStackFrame) {
    let scancode: u8 = unsafe { crate::arch::port::Port::new(0x60).read() };
    crate::drivers::char::keyboard::push_scancode(scancode);
    send_eoi(PIC1_OFFSET + 1);
}
