fn unique_marker(stdout: &[u8]) -> Result<&[u8], DistributionError> {
    let positions = stdout
        .windows(MARKER.len())
        .enumerate()
        .filter_map(|(index, window)| (window == MARKER).then_some(index))
        .collect::<Vec<_>>();
    if positions.len() != 1 {
        return Err(error(DistributionErrorId::ObjectUnavailable));
    }
    let start = positions[0] + MARKER.len();
    let end = stdout[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(stdout.len(), |offset| start + offset);
    let value = &stdout[start..end];
    if value.is_empty() {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(value)
}

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
