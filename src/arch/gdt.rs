use core::arch::asm;
use crate::arch::types::{GdtPointer, GdtEntry, TaskStateSegment, Gdt};

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
