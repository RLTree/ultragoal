fn verify_file<F>(
    path: &Path,
    artifact: &ArtifactRecord,
    after_open: &mut F,
) -> Result<FileIdentity, OrchestrationError>
where
    F: FnMut(&Path),
{
    let before = fs::symlink_metadata(path).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
    if !before.is_file() || before.len() > MAX_ARTIFACT_BYTES {
        return Err(OrchestrationError::InvalidWorkerResult);
    }
    let mut file = File::open(path).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
    let opened = file
        .metadata()
        .map_err(|_| OrchestrationError::InvalidWorkerResult)?;
    let identity = file_identity(&opened)?;
    if file_identity(&before)? != identity || hard_link_count(&before)? != 1 {
        return Err(OrchestrationError::InvalidWorkerResult);
    }
    after_open(path);
    let (digest, bytes) = hash_file(&mut file)?;
    let handle_after = file
        .metadata()
        .map_err(|_| OrchestrationError::InvalidWorkerResult)?;
    let path_after =
        fs::symlink_metadata(path).map_err(|_| OrchestrationError::InvalidWorkerResult)?;
    if path_after.file_type().is_symlink()
        || file_identity(&handle_after)? != identity
        || file_identity(&path_after)? != identity
        || hard_link_count(&handle_after)? != 1
        || hard_link_count(&path_after)? != 1
        || handle_after.len() != before.len()
        || handle_after.modified().ok() != before.modified().ok()
        || bytes != artifact.byte_length
        || digest != artifact.sha256
    {
        return Err(OrchestrationError::InvalidWorkerResult);
    }
    Ok(identity)
}

fn hash_file(file: &mut File) -> Result<(String, u64), OrchestrationError> {
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| OrchestrationError::InvalidWorkerResult)?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .ok_or(OrchestrationError::ResourceLimit)?;
        if bytes > MAX_ARTIFACT_BYTES {
            return Err(OrchestrationError::ResourceLimit);
        }
        hasher.update(&buffer[..read]);
    }
    Ok((format!("sha256:{:x}", hasher.finalize()), bytes))
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct FileIdentity(u64, u64);

#[cfg(unix)]
fn file_identity(metadata: &Metadata) -> Result<FileIdentity, OrchestrationError> {
    use std::os::unix::fs::MetadataExt;
    Ok(FileIdentity(metadata.dev(), metadata.ino()))
}

#[cfg(unix)]
fn hard_link_count(metadata: &Metadata) -> Result<u64, OrchestrationError> {
    use std::os::unix::fs::MetadataExt;
    Ok(metadata.nlink())
}

#[cfg(not(unix))]
fn file_identity(_: &Metadata) -> Result<FileIdentity, OrchestrationError> {
    Err(OrchestrationError::InvalidWorkerResult)
}

#[cfg(not(unix))]
fn hard_link_count(_: &Metadata) -> Result<u64, OrchestrationError> {
    Err(OrchestrationError::InvalidWorkerResult)
}
