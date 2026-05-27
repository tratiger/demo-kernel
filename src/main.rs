#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

mod port;
mod serial;
mod gdt;
mod mem;
mod interrupts;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

// Multiboot constants
const ALIGN: u32 = 1 << 0;
const MEMINFO: u32 = 1 << 1;
const MAGIC: u32 = 0x1BADB002;
const FLAGS: u32 = ALIGN | MEMINFO;
const CHECKSUM: u32 = -(MAGIC as i32 + FLAGS as i32) as u32;

// The boot assembly
core::arch::global_asm!(
    ".section .multiboot_header",
    ".align 4",
    ".long 0x1BADB002", // MAGIC
    ".long 0x00000003", // FLAGS (ALIGN | MEMINFO)
    ".long -0x1BADB005", // CHECKSUM
    ".section .bss",
    ".align 16",
    ".global stack_top",
    "stack_bottom:",
    ".skip 16384", // 16KB
    "stack_top:",
    ".section .text",
    ".global _start",
    ".type _start, @function",
    "_start:",
    "mov esp, offset stack_top",
    "call kernel_main",
    "cli",
    "1:",
    "hlt",
    "jmp 1b",
);

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    crate::serial::SERIAL1.lock().init();

    println!("Loading GDT...");
    gdt::init();
    println!("GDT Loaded Successfully!");

    println!("Loading IDT...");
    interrupts::init_idt();
    println!("IDT Loaded Successfully!");

    println!("Testing Breakpoint...");
    unsafe {
        core::arch::asm!("int 3", options(nomem, nostack));
    }

    println!("Initializing PIC...");
    interrupts::init_pic();
    println!("PIC Initialized!");

    println!("Enabling Interrupts...");
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }

    println!("Hello, Rust OS World! Hex: {:#X}", 0xDEADBEEFu32);

    loop {
        // Just hang here
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
