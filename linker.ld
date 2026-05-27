ENTRY(_start)

SECTIONS {
    /* Set the load address to 1MB, a conventional place for kernels to load */
    . = 1M;

    /* Multiboot header must be within the first 8KB of the file */
    .text ALIGN(4K) : {
        /* Keep the multiboot header so the linker doesn't garbage collect it */
        KEEP(*(.multiboot_header))
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : {
        *(.rodata .rodata.*)
    } :text

    .data ALIGN(4K) : {
        *(.data .data.*)
    } :data

    .bss ALIGN(4K) : {
        *(.bss .bss.*)
        *(COMMON)
    } :data
}
