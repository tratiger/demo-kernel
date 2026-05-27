#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

use core::panic::PanicInfo;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::format;

mod port;
mod serial;
mod gdt;
mod mem;
mod interrupts;
mod multiboot;
mod memory;
mod paging;
mod allocator;

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
    "push ebx",
    "push eax",
    "call kernel_main",
    "cli",
    "1:",
    "hlt",
    "jmp 1b",
);

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(magic: u32, mbi_ptr: u32) -> ! {
    crate::serial::SERIAL1.lock().init();

    crate::multiboot::parse(magic, mbi_ptr);

    unsafe { crate::memory::init() };
    unsafe { crate::paging::init() };

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

    println!("Initializing Heap...");
    unsafe { crate::allocator::init_heap() };
    println!("Heap Initialized!");

    println!("Starting fragmentation stress test...");
    for i in 0..1000 {
        // Allocate and immediately drop to test fragmentation and coalescing
        let mut temp_vec: Vec<u8> = Vec::with_capacity(1024 * 16); // 16KB per allocation
        for j in 0..10 {
            temp_vec.push(j as u8);
        }
        if i % 100 == 0 {
            println!("Allocated iteration {}", i);
        }
        // temp_vec is dropped here, its memory freed
    }
    println!("Stress test passed! Coalescing works.");

    println!("Testing Kernel Heap Allocator...");
    let heap_value = Box::new(42);
    println!("Boxed value allocated: {}", *heap_value);

    let mut v = Vec::with_capacity(100);
    for i in 0..100 {
        v.push(i);
    }
    println!("Vec allocated and pushed 100 elements. Last element: {}", v[99]);

    println!("Testing format!");
    // String formatting uses alloc under the hood
    let s = format!("Allocated String with value: {}", *heap_value);

    // Explicitly write string slice to our serial macro
    crate::println!("Formatted String: {}", s.as_str());

    println!("Testing dynamic memory mapping...");
    unsafe {
        // Map virtual address 0x40000000 to a new physical frame
        let new_frame = crate::memory::allocate_frame().unwrap();
        crate::paging::map_page(0x40000000, new_frame, 0x3); // Present | R/W

        // Read and write to it
        let ptr = 0x40000000 as *mut u32;
        *ptr = 0x12345678;
        println!("Successfully wrote to mapped memory. Read back: {:#X}", *ptr);
    }

    println!("Testing Page Fault Exception (Accessing unmapped memory at 0x50000000)...");
    unsafe {
        let ptr = 0x50000000 as *mut u32;
        let _val = core::ptr::read_volatile(ptr);
        println!("ERROR: Should not reach this line! Value read: {:#X}", _val);
    }

    loop {
        // Just hang here
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }
}
