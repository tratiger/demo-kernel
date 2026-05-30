pub mod memory;
pub mod allocator;
pub mod mem;
pub mod types;

pub fn is_user_memory(start: u32, len: u32) -> bool {
    let end = start.checked_add(len);
    if end.is_none() {
        return false;
    }
    let end = end.unwrap();

    // Valid data segment
    let valid_data = start >= 0x40000000 && end <= 0x40000000 + 40 * 4096;

    // Valid stack segment
    let valid_stack = start >= 0xA0000000 - 40 * 4096 && end <= 0xA0000000;

    // Additionally, could be a string literal, maybe we just do:
    let base_valid = start >= 0x40000000 && end <= 0xA0000000;

    base_valid // Or true validation logic depending on how strict you want to be
}
