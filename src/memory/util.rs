pub unsafe fn memsetw(mut destination: *mut u16, value: u16, count: usize) {
    for _ in 0..count {
        *destination = value;
        destination = destination.add(1);
    }
}
