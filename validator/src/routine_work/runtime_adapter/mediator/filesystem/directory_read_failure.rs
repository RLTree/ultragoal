use super::*;

#[cfg(unix)]
pub(crate) fn readdir_failed() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { *libc::__error() != 0 }
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe { *libc::__errno_location() != 0 }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "android")))]
    {
        false
    }
}

#[cfg(unix)]
pub(crate) struct DirectoryStream(pub(crate) *mut libc::DIR);

#[cfg(unix)]
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(test)]
pub(crate) type CaptureHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
pub(crate) fn capture_hook() -> &'static Mutex<Option<CaptureHook>> {
    static HOOK: OnceLock<Mutex<Option<CaptureHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_test_output_capture_hook(hook: impl FnOnce() + Send + 'static) {
    *capture_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn run_test_capture_hook() {
    if let Some(hook) = capture_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(not(test))]
pub(crate) fn run_test_capture_hook() {}

pub(crate) fn digest_reader(reader: &mut File, limit: u64) -> Result<String, RoutineError> {
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| mediator_error("mediator-file-read-failed"))?;
        if read == 0 {
            break;
        }
        observed = observed
            .checked_add(read as u64)
            .filter(|total| *total <= limit)
            .ok_or_else(|| mediator_error("mediator-file-read-limit-exceeded"))?;
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub(crate) fn decode_path(value: &str) -> Result<PathBuf, RoutineError> {
    if value.is_empty()
        || value.len() % 2 != 0
        || value.len() > 16_384
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(mediator_error("mediator-executable-path-encoding-invalid"));
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| mediator_error("mediator-executable-path-encoding-invalid"))?;
            u8::from_str_radix(text, 16)
                .map_err(|_| mediator_error("mediator-executable-path-encoding-invalid"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(OsString::from_vec(bytes)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes)
            .map(PathBuf::from)
            .map_err(|_| mediator_error("mediator-executable-path-encoding-invalid"))
    }
}
