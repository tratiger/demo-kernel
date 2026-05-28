#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::vec::Vec;
use core::panic::PanicInfo;

pub mod arch;
pub mod drivers;
pub mod fs;
pub mod kernel;
pub mod mm;
mod multiboot;

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
    ".long 0x1BADB002",  // MAGIC
    ".long 0x00000003",  // FLAGS (ALIGN | MEMINFO)
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
    crate::drivers::char::serial::SERIAL1.lock().init();

    let initrd_info = crate::multiboot::parse(magic, mbi_ptr);

    unsafe { crate::mm::memory::init(initrd_info) };
    unsafe { crate::arch::paging::init() };

    println!("Loading GDT...");
    crate::arch::gdt::init();
    println!("GDT Loaded Successfully!");

    println!("Loading IDT...");
    crate::arch::interrupts::init_idt();
    println!("IDT Loaded Successfully!");

    println!("Testing Breakpoint...");
    unsafe {
        core::arch::asm!("int 3", options(nomem, nostack));
    }

    println!("Initializing PIC...");
    crate::arch::interrupts::init_pic();
    println!("PIC Initialized!");

    println!("Enabling Interrupts...");
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
    }

    println!("Hello, Rust OS World! Hex: {:#X}", 0xDEADBEEFu32);

    println!("Initializing Heap...");
    unsafe { crate::mm::allocator::init_heap() };
    println!("Heap Initialized!");

    if let Some((start, end)) = initrd_info {
        crate::drivers::fs::initrd::init(start, end);
    }

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
    println!(
        "Vec allocated and pushed 100 elements. Last element: {}",
        v[99]
    );

    println!("Testing format!");
    // String formatting uses alloc under the hood
    let s = format!("Allocated String with value: {}", *heap_value);

    // Explicitly write string slice to our serial macro
    crate::println!("Formatted String: {}", s.as_str());

    println!("Initializing Scheduler...");
    crate::kernel::task::init();

    println!("Testing dynamic memory mapping with user privilege...");
    unsafe {
        for i in 0..40 {
            let frame = crate::mm::memory::allocate_frame().unwrap();
            crate::arch::paging::map_page(
                0x40000000 + i * 4096,
                frame,
                0x3 | crate::arch::paging::USER_ACCESSIBLE,
            );
        }
        for i in 0..40 {
            let frame = crate::mm::memory::allocate_frame().unwrap();
            crate::arch::paging::map_page(
                0xA0000000 - i * 4096,
                frame,
                0x3 | crate::arch::paging::USER_ACCESSIBLE,
            );
        }

        // Map virtual address 0xA0000000 to a new physical frame (User Stack)
        let new_frame_stack = crate::mm::memory::allocate_frame().unwrap();
        crate::arch::paging::map_page(
            0xA0003000,
            new_frame_stack,
            0x3 | crate::arch::paging::USER_ACCESSIBLE,
        );

        // Setup interactive shell payload
        let buf_addr: u32 = 0x40000000 + 4000;
        let line_buf_addr: u32 = 0x40000000 + 3000;
        let prompt_addr: u32 = 0x40000000 + 2000;
        let prompt = b"OMU-OS> ";
        let help_msg = b"Supported commands: help, ls, cat <file>
";
        let help_msg_addr: u32 = 0x40000000 + 2100;
        let ls_path = b"/";
        let ls_path_addr: u32 = 0x40000000 + 2200;

        let mut user_code_payload = Vec::new();

        macro_rules! emit { ($($b:expr),*) => { $(user_code_payload.push($b);)* }; }
        macro_rules! emit_u32 {
            ($v:expr) => {
                user_code_payload.extend_from_slice(&$v.to_le_bytes());
            };
        }

        let loop_start = user_code_payload.len();

        // 1. sys_write prompt
        emit!(0xB8);
        emit_u32!(4u32); // mov eax, 4
        emit!(0xBB);
        emit_u32!(1u32); // mov ebx, 1
        emit!(0xB9);
        emit_u32!(prompt_addr); // mov ecx, prompt_addr
        emit!(0xBA);
        emit_u32!(prompt.len() as u32); // mov edx, len
        emit!(0xCD, 0x80); // int 0x80

        // line_len = 0 (use ebp)
        emit!(0xBD);
        emit_u32!(0u32); // mov ebp, 0

        let read_loop_start = user_code_payload.len();

        // 2. sys_read 1 byte
        emit!(0xB8);
        emit_u32!(3u32); // mov eax, 3
        emit!(0xBB);
        emit_u32!(0u32); // mov ebx, 0 (stdin)
        emit!(0xB9);
        emit_u32!(buf_addr); // mov ecx, buf_addr
        emit!(0xBA);
        emit_u32!(1u32); // mov edx, 1
        emit!(0xCD, 0x80); // int 0x80

        // 3. sys_write echo 1 byte
        emit!(0xB8);
        emit_u32!(4u32); // mov eax, 4
        emit!(0xBB);
        emit_u32!(1u32); // mov ebx, 1 (stdout)
        emit!(0xB9);
        emit_u32!(buf_addr); // mov ecx, buf_addr
        emit!(0xBA);
        emit_u32!(1u32); // mov edx, 1
        emit!(0xCD, 0x80); // int 0x80

        emit!(0xA0);
        emit_u32!(buf_addr); // mov al, [buf_addr]

        emit!(0x3C, 0x0A); // cmp al, 10
        emit!(0x0F, 0x84); // je process
        let je_process = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0x3C, 0x0D); // cmp al, 13
        emit!(0x0F, 0x84); // je process
        let je_process2 = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0x89, 0xE9); // mov ecx, ebp
        emit!(0x81, 0xC1);
        emit_u32!(line_buf_addr); // add ecx, line_buf_addr
        emit!(0x88, 0x01); // mov [ecx], al
        emit!(0x45); // inc ebp

        emit!(0xE9);
        let current = user_code_payload.len() + 4;
        emit_u32!((read_loop_start as isize - current as isize) as u32);

        let process_start = user_code_payload.len();
        let off = (process_start as isize - (je_process + 4) as isize) as u32;
        user_code_payload[je_process..je_process + 4].copy_from_slice(&off.to_le_bytes());
        let off = (process_start as isize - (je_process2 + 4) as isize) as u32;
        user_code_payload[je_process2..je_process2 + 4].copy_from_slice(&off.to_le_bytes());

        // null terminate
        emit!(0x89, 0xE9); // mov ecx, ebp
        emit!(0x81, 0xC1);
        emit_u32!(line_buf_addr); // add ecx, line_buf_addr
        emit!(0xC6, 0x01, 0x00); // mov byte ptr [ecx], 0

        // check empty
        emit!(0x85, 0xED); // test ebp, ebp
        emit!(0x0F, 0x84); // je loop_start
        let je_empty = user_code_payload.len();
        emit_u32!(0u32);

        // check help
        emit!(0x83, 0xFD, 0x04); // cmp ebp, 4
        emit!(0x0F, 0x85);
        let jne_ls = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0x81, 0x3D);
        emit_u32!(line_buf_addr);
        emit_u32!(u32::from_le_bytes([b'h', b'e', b'l', b'p']));
        emit!(0x0F, 0x85);
        let jne_ls2 = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0xB8);
        emit_u32!(4u32);
        emit!(0xBB);
        emit_u32!(1u32);
        emit!(0xB9);
        emit_u32!(help_msg_addr);
        emit!(0xBA);
        emit_u32!(help_msg.len() as u32);
        emit!(0xCD, 0x80);

        emit!(0xE9);
        let current = user_code_payload.len() + 4;
        emit_u32!((loop_start as isize - current as isize) as u32);

        let ls_start = user_code_payload.len();
        let off = (ls_start as isize - (jne_ls + 4) as isize) as u32;
        user_code_payload[jne_ls..jne_ls + 4].copy_from_slice(&off.to_le_bytes());
        let off = (ls_start as isize - (jne_ls2 + 4) as isize) as u32;
        user_code_payload[jne_ls2..jne_ls2 + 4].copy_from_slice(&off.to_le_bytes());

        // check ls
        emit!(0x83, 0xFD, 0x02); // cmp ebp, 2
        emit!(0x0F, 0x85);
        let jne_cat = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0x66, 0x81, 0x3D);
        emit_u32!(line_buf_addr);
        emit!(b'l', b's');
        emit!(0x0F, 0x85);
        let jne_cat2 = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0xB8);
        emit_u32!(5u32);
        emit!(0xBB);
        emit_u32!(ls_path_addr);
        emit!(0xB9);
        emit_u32!(1u32);
        emit!(0xCD, 0x80);

        emit!(0x89, 0xC3);
        emit!(0xB8);
        emit_u32!(6u32);
        emit!(0xB9);
        emit_u32!(buf_addr);
        emit!(0xBA);
        emit_u32!(1000u32);
        emit!(0xCD, 0x80);

        emit!(0x89, 0xC2); // mov edx, eax
        emit!(0x85, 0xD2);
        emit!(0x0F, 0x84);
        let jz_print_ls = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0x31, 0xC9);
        let replace_loop = user_code_payload.len();
        emit!(0x80, 0xB9);
        emit_u32!(buf_addr);
        emit!(0x00);
        emit!(0x75, 0x07);
        emit!(0xC6, 0x81);
        emit_u32!(buf_addr);
        emit!(0x0A); // 10 is newline
        emit!(0x41);
        emit!(0x39, 0xD1);
        emit!(0x72);
        let current = user_code_payload.len() + 1;
        let jump_back = (replace_loop as isize - current as isize) as u8;
        emit!(jump_back);

        let print_ls = user_code_payload.len();
        let off = (print_ls as isize - (jz_print_ls + 4) as isize) as u32;
        user_code_payload[jz_print_ls..jz_print_ls + 4].copy_from_slice(&off.to_le_bytes());

        emit!(0xB8);
        emit_u32!(4u32);
        emit!(0xBB);
        emit_u32!(1u32);
        emit!(0xB9);
        emit_u32!(buf_addr);
        emit!(0xCD, 0x80);

        // write newline after ls
        emit!(0xB8);
        emit_u32!(4u32);
        emit!(0xBB);
        emit_u32!(1u32);
        emit!(0xC6, 0x05);
        emit_u32!(buf_addr);
        emit!(0x0A);
        emit!(0xB9);
        emit_u32!(buf_addr);
        emit!(0xBA);
        emit_u32!(1u32);
        emit!(0xCD, 0x80);

        emit!(0xE9);
        let current = user_code_payload.len() + 4;
        emit_u32!((loop_start as isize - current as isize) as u32);

        let cat_start = user_code_payload.len();
        let off = (cat_start as isize - (jne_cat + 4) as isize) as u32;
        user_code_payload[jne_cat..jne_cat + 4].copy_from_slice(&off.to_le_bytes());
        let off = (cat_start as isize - (jne_cat2 + 4) as isize) as u32;
        user_code_payload[jne_cat2..jne_cat2 + 4].copy_from_slice(&off.to_le_bytes());

        // check cat
        emit!(0x83, 0xFD, 0x04); // cmp ebp, 4
        emit!(0x0F, 0x8E);
        let jle_invalid = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0x81, 0x3D);
        emit_u32!(line_buf_addr);
        emit_u32!(u32::from_le_bytes([b'c', b'a', b't', b' ']));
        emit!(0x0F, 0x85);
        let jne_invalid = user_code_payload.len();
        emit_u32!(0u32);

        emit!(0xB8);
        emit_u32!(5u32); // sys_open
        emit!(0xBB);
        emit_u32!(line_buf_addr + 4);
        emit!(0x89, 0xE9);
        emit!(0x83, 0xE9, 0x04); // len - 4
        emit!(0xCD, 0x80);

        emit!(0x89, 0xC3); // fd
        emit!(0xB8);
        emit_u32!(3u32); // sys_read
        emit!(0xB9);
        emit_u32!(buf_addr);
        emit!(0xBA);
        emit_u32!(1000u32);
        emit!(0xCD, 0x80);

        emit!(0x89, 0xC2); // size read
        emit!(0xB8);
        emit_u32!(4u32); // sys_write
        emit!(0xBB);
        emit_u32!(1u32);
        emit!(0xB9);
        emit_u32!(buf_addr);
        emit!(0xCD, 0x80);

        emit!(0xB8);
        emit_u32!(4u32); // sys_write newline
        emit!(0xBB);
        emit_u32!(1u32);
        emit!(0xC6, 0x05);
        emit_u32!(buf_addr);
        emit!(0x0A);
        emit!(0xB9);
        emit_u32!(buf_addr);
        emit!(0xBA);
        emit_u32!(1u32);
        emit!(0xCD, 0x80);

        let invalid_start = user_code_payload.len();
        let off = (invalid_start as isize - (jle_invalid + 4) as isize) as u32;
        user_code_payload[jle_invalid..jle_invalid + 4].copy_from_slice(&off.to_le_bytes());
        let off = (invalid_start as isize - (jne_invalid + 4) as isize) as u32;
        user_code_payload[jne_invalid..jne_invalid + 4].copy_from_slice(&off.to_le_bytes());
        let off = (invalid_start as isize - (je_empty + 4) as isize) as u32;
        user_code_payload[je_empty..je_empty + 4].copy_from_slice(&off.to_le_bytes());

        emit!(0xE9);
        let current = user_code_payload.len() + 4;
        emit_u32!((loop_start as isize - current as isize) as u32);

        let ptr = 0x40000000 as *mut u8;
        core::ptr::copy_nonoverlapping(user_code_payload.as_ptr(), ptr, user_code_payload.len());
        core::ptr::copy_nonoverlapping(prompt.as_ptr(), prompt_addr as *mut u8, prompt.len());
        core::ptr::copy_nonoverlapping(help_msg.as_ptr(), help_msg_addr as *mut u8, help_msg.len());
        core::ptr::copy_nonoverlapping(ls_path.as_ptr(), ls_path_addr as *mut u8, ls_path.len());

        println!("Successfully deployed user payload to 0x40000000.");

        println!("Jumping to usermode...");
        crate::kernel::task::jump_to_usermode(0x40000000, 0xA0004000);
    }

    // We should not reach here since we jump to usermode
    loop {}
}
