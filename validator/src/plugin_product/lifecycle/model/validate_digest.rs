pub(super) fn validate_digest(value: &str) -> Result<(), LifecycleError> {
    let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
        return Err(LifecycleError::InvalidDigest);
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(LifecycleError::InvalidDigest);
    }
    Ok(())
}
