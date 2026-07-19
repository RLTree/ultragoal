use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicU64, Ordering};

static OUTPUT_GENERATION: AtomicU64 = AtomicU64::new(0);

fn publish_public_result(
    output: Option<&crate::context::AuthorizedPath>,
    bytes: &[u8],
) -> Result<(), ProductionRuntimeError> {
    let Some(output) = output else {
        return Ok(());
    };
    output
        .revalidate()
        .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-invalid"))?;
    let path = &output.canonical_path;
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() && metadata.nlink() == 1 => {
            return (std::fs::read(path).ok().as_deref() == Some(bytes))
                .then_some(())
                .ok_or_else(|| ProductionRuntimeError::new("evaluation-run-output-conflict"));
        }
        Ok(_) => {
            return Err(ProductionRuntimeError::new(
                "evaluation-run-output-conflict",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(ProductionRuntimeError::new("evaluation-run-output-invalid")),
    }
    let parent = path
        .parent()
        .ok_or_else(|| ProductionRuntimeError::new("evaluation-run-output-invalid"))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ProductionRuntimeError::new("evaluation-run-output-invalid"))?;
    let temporary = parent.join(format!(
        ".{name}.evaluation-output-{}-{}",
        std::process::id(),
        OUTPUT_GENERATION.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-publish-failed"))?;
        use std::io::Write;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-publish-failed"))?;
        std::fs::hard_link(&temporary, path)
            .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-conflict"))?;
        std::fs::remove_file(&temporary)
            .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-publish-failed"))?;
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| ProductionRuntimeError::new("evaluation-run-output-publish-failed"))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
