pub const fn grow_capacity(capacity: usize) -> usize {
    if capacity < 8 {
        8
    } else {
        capacity * 2
    }
}

pub unsafe fn memcmp(ptr1: *const u8, ptr2: *const u8, n: usize) -> i32 {
    for i in 0..n {
        if *ptr1.add(i) != *ptr2.add(i) {
            return (*ptr1.add(i) as i32) - (*ptr2.add(i) as i32);
        }
    }
    0
}
