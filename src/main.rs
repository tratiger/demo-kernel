#![no_std]
#![no_main]

use core::panic::PanicInfo;

mod serial;

use serial::SerialPort;

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
    ".intel_syntax noprefix",
    ".section .multiboot_header",
    ".align 4",
    ".long 0x1BADB002", // MAGIC
    ".long 0x00000003", // FLAGS (ALIGN | MEMINFO)
    ".long -0x1BADB005", // CHECKSUM
    ".section .bss",
    ".align 16",
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
    let com1 = SerialPort::new(0x3F8);
    com1.init();

    com1.write_byte(b'A');

    loop {
        // Just hang here
    }
}
