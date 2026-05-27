#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        unsafe {
            *dest.add(i) = *src.add(i);
        }
        i += 1;
    }
    dest
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        unsafe {
            *s.add(i) = c as u8;
        }
        i += 1;
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        unsafe {
            let a = *s1.add(i);
            let b = *s2.add(i);
            if a != b {
                return (a as i32) - (b as i32);
            }
        }
        i += 1;
    }
    0
}