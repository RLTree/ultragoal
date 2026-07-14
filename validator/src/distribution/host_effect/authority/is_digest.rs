fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn zeroize<const N: usize>(bytes: &mut [u8; N]) {
    for byte in bytes {
        // SAFETY: each pointer comes from a unique mutable slice element. The
        // volatile store and fence prevent elision of this best-effort wipe.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

const fn authority_error(id: HostEffectAuthorityErrorId) -> HostEffectAuthorityError {
    HostEffectAuthorityError { id }
}
