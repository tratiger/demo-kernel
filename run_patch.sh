#!/bin/bash
patch -p1 << 'PATCH_EOF'
--- a/src/main.rs
+++ b/src/main.rs
@@ -8,6 +8,8 @@
 mod gdt;
 mod mem;
 mod interrupts;
+mod multiboot;
+mod memory;
+mod paging;

 #[panic_handler]
 fn panic(_info: &PanicInfo) -> ! {
@@ -39,6 +41,8 @@
     ".type _start, @function",
     "_start:",
     "mov esp, offset stack_top",
+    "push ebx",
+    "push eax",
     "call kernel_main",
     "cli",
     "1:",
@@ -47,8 +51,13 @@
 );

 #[unsafe(no_mangle)]
-pub extern "C" fn kernel_main() -> ! {
+pub extern "C" fn kernel_main(magic: u32, mbi_ptr: u32) -> ! {
     crate::serial::SERIAL1.lock().init();
+
+    crate::multiboot::parse(magic, mbi_ptr);
+
+    unsafe { crate::memory::init() };
+    unsafe { crate::paging::init() };

     println!("Loading GDT...");
     gdt::init();
PATCH_EOF
