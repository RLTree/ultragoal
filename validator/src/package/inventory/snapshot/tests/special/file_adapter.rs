use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) struct SpecialFileRequest<'a> {
    pub(super) path: &'a Path,
    pub(super) mode: u32,
}

pub(super) struct SpecialFileResponse;

#[derive(Debug)]
pub(super) enum SpecialFileError {
    InvalidPath,
    CreateRejected,
}

pub(super) fn create_fifo(
    request: SpecialFileRequest<'_>,
) -> Result<SpecialFileResponse, SpecialFileError> {
    let name = CString::new(request.path.as_os_str().as_bytes())
        .map_err(|_| SpecialFileError::InvalidPath)?;
    if unsafe { libc::mkfifo(name.as_ptr(), request.mode as libc::mode_t) } != 0 {
        return Err(SpecialFileError::CreateRejected);
    }
    Ok(SpecialFileResponse)
}
