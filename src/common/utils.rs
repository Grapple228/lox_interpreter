pub const fn grow_capacity(capacity: usize) -> usize {
    if capacity < 8 {
        8
    } else {
        capacity * 2
    }
}
