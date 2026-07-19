fn publish_public_result(
    output_path: Option<&std::path::Path>,
    bytes: &[u8],
) -> Result<(), ProductionRuntimeError> {
    let Some(path) = output_path else {
        return Ok(());
    };
    if let Ok(existing) = std::fs::read(path) {
        return (existing == bytes)
            .then_some(())
            .ok_or_else(|| ProductionRuntimeError::new("evaluation-run-output-conflict"));
    }
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-publish-failed"))?;
    use std::io::Write;
    output
        .write_all(bytes)
        .and_then(|()| output.sync_all())
        .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-publish-failed"))
}
