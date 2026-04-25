pub fn rdrand64() -> u64 {
    let mut val: u64;
    unsafe {
        core::arch::asm!(
            "2: rdrand {val}",
            "   jnc 2b",
            val = out(reg) val,
        );
    }
    val
}

pub fn fill_bytes(buf: &mut [u8]) {
    let mut i = 0;
    while i < buf.len() {
        let r = rdrand64();
        let bytes = r.to_le_bytes();
        let remaining = buf.len() - i;
        let chunk = remaining.min(8);
        buf[i..i + chunk].copy_from_slice(&bytes[..chunk]);
        i += chunk;
    }
}
