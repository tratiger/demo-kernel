use core::arch::asm;

use crate::arch::types::{PageDirectory, PageTable};

pub const PAGE_SIZE: u32 = 4096;
pub const USER_ACCESSIBLE: u32 = 1 << 2;

pub static mut KERNEL_PAGE_DIRECTORY: PageDirectory = PageDirectory::new();
pub static mut PAGE_TABLE_0: PageTable = PageTable::new();
pub static mut PAGE_TABLE_1: PageTable = PageTable::new();

pub unsafe fn init() {
    let pd_ptr = core::ptr::addr_of_mut!(KERNEL_PAGE_DIRECTORY);
    let pt0_ptr = core::ptr::addr_of_mut!(PAGE_TABLE_0);
    let pt1_ptr = core::ptr::addr_of_mut!(PAGE_TABLE_1);

    unsafe {
        // Identity map the first 8MB
        for i in 0..1024 {
            // First 4MB
            let phys_addr_0 = (i as u32) * PAGE_SIZE;
            // Flags: Present (0x01) | Read/Write (0x02)
            (*pt0_ptr).entries[i] = phys_addr_0 | 3;

            // Second 4MB
            let phys_addr_1 = ((i + 1024) as u32) * PAGE_SIZE;
            (*pt1_ptr).entries[i] = phys_addr_1 | 3;
        }

        // Register the page tables in the page directory
        let pt0_phys = pt0_ptr as u32;
        let pt1_phys = pt1_ptr as u32;

        (*pd_ptr).entries[0] = pt0_phys | 3; // Present, R/W
        (*pd_ptr).entries[1] = pt1_phys | 3; // Present, R/W

        // Recursive paging
        let pd_phys = pd_ptr as u32;
        (*pd_ptr).entries[1023] = pd_phys | 3;

        // Enable paging

        asm!("mov cr3, {}", in(reg) pd_phys, options(nostack, preserves_flags));

        let mut cr0: u32;
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
        cr0 |= 0x80000000; // Set PG bit (bit 31)
        asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));
    }

    crate::println!("Paging enabled successfully!");
}

pub unsafe fn map_page(virt_addr: u32, phys_addr: u32, flags: u32) {
    let pd_index = (virt_addr >> 22) as usize;
    let pt_index = ((virt_addr >> 12) & 0x3FF) as usize;

    let pd_virt = 0xFFFFF000 as *mut u32;
    let pde = unsafe { core::ptr::read_volatile(pd_virt.add(pd_index)) };

    if (pde & 1) == 0 {
        // Page table not present, allocate a new frame
        let new_frame = unsafe {
            crate::mm::memory::allocate_frame().expect("Out of memory when allocating page table!")
        };

        // Add the new page table to the page directory (Present, R/W)
        unsafe {
            core::ptr::write_volatile(pd_virt.add(pd_index), new_frame | 3 | (flags & USER_ACCESSIBLE));
        }

        // Clear the new page table
        let pt_virt = (0xFFC00000 + (pd_index as u32 * 4096)) as *mut u32;
        unsafe {
            core::ptr::write_bytes(pt_virt, 0, 1024);
        }
    }

    let pt_virt = (0xFFC00000 + (pd_index as u32 * 4096)) as *mut u32;
    // Set the page table entry
    unsafe {
        core::ptr::write_volatile(pt_virt.add(pt_index), (phys_addr & 0xFFFFF000) | (flags & 0xFFF));
    }

    // Flush TLB
    unsafe {
        asm!("invlpg [{}]", in(reg) virt_addr, options(nostack, preserves_flags));
    }
}
