fn open_promotion_input(
    root: &std::fs::File,
    relative_path: &str,
) -> Result<std::fs::File, EvaluationError> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    if !safe_relative_path(relative_path) {
        return Err(EvaluationError::new(
            "evaluation-review-evidence-path-invalid",
        ));
    }
    let components = std::path::Path::new(relative_path)
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => value,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    let mut directory = root
        .try_clone()
        .map_err(|_| EvaluationError::new("evaluation-review-evidence-root-changed"))?;
    for (index, component) in components.iter().enumerate() {
        let component = std::ffi::CString::new(component.as_bytes())
            .map_err(|_| EvaluationError::new("evaluation-review-evidence-path-invalid"))?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if index + 1 == components.len() {
                0
            } else {
                libc::O_DIRECTORY
            };
        let descriptor = unsafe { libc::openat(directory.as_raw_fd(), component.as_ptr(), flags) };
        if descriptor < 0 {
            return Err(EvaluationError::new(
                "evaluation-review-evidence-open-failed",
            ));
        }
        let opened = unsafe { std::fs::File::from_raw_fd(descriptor) };
        if index + 1 == components.len() {
            return Ok(opened);
        }
        directory = opened;
    }
    Err(EvaluationError::new(
        "evaluation-review-evidence-path-invalid",
    ))
}

fn read_promotion_input(file: &std::fs::File, length: u64) -> Result<String, EvaluationError> {
    use std::os::unix::fs::FileExt;
    let mut bytes = vec![0_u8; length as usize];
    let mut offset = 0;
    while offset < bytes.len() {
        let read = file
            .read_at(&mut bytes[offset..], offset as u64)
            .map_err(|_| EvaluationError::new("evaluation-review-evidence-read-failed"))?;
        if read == 0 {
            return Err(EvaluationError::new(
                "evaluation-review-evidence-read-truncated",
            ));
        }
        offset += read;
    }
    Ok(digest(&bytes))
}
