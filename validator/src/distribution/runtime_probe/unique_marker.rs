fn read_output<T: Read>(stream: Option<T>) -> Result<Vec<u8>, DistributionError> {
    let mut bytes = Vec::new();
    stream
        .ok_or_else(|| error(DistributionErrorId::EffectFailed))?
        .take(OUTPUT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(DistributionErrorId::EffectFailed))?;
    if bytes.len() > OUTPUT_LIMIT {
        return Err(error(DistributionErrorId::ObjectTooLarge));
    }
    Ok(bytes)
}
