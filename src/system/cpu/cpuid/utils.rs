pub fn cpuid_bits_needed(count: u8) -> u8 {
    let mut mask: u8 = 0x80;
    let mut cnt: u8 = 8;

    while (cnt > 0) && ((mask & count) != mask) {
        mask >>= 1;
        cnt -= 1;
    }

    cnt
}
