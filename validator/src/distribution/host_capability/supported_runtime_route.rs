pub(crate) const SUPPORTED_RUNTIME_PROGRAM: &str = "plugins/harness-ultragoal/runtime/ultragoal";

pub(crate) fn supported_runtime_program(
    path: &Path,
    home: &Path,
) -> Result<bool, DistributionError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if !canonical.starts_with(home) || !executable(path)? {
        return Ok(false);
    }
    Ok(canonical
        .strip_prefix(home)
        .ok()
        .and_then(|row| row.to_str())
        == Some(SUPPORTED_RUNTIME_PROGRAM))
}
