use super::*;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

pub(crate) fn immutable_routine_program_matches(
    path_hex: &str,
    sha256: &str,
    byte_length: u64,
    unix_mode: Option<u32>,
    device: u64,
    inode: u64,
    changed_seconds: i64,
    changed_nanos: i64,
) -> Result<PathBuf, RoutineError> {
    let bytes = (0..path_hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&path_hex[index..index + 2], 16))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| adapter_error("adapter-runner-program-path-invalid"))?;
    #[cfg(unix)]
    let path = PathBuf::from(std::ffi::OsString::from_vec(bytes));
    #[cfg(not(unix))]
    let path = PathBuf::from(
        String::from_utf8(bytes)
            .map_err(|_| adapter_error("adapter-runner-program-path-invalid"))?,
    );
    mediator::validate_bound_routine_program(
        &path,
        sha256,
        byte_length,
        unix_mode,
        device,
        inode,
        changed_seconds,
        changed_nanos,
    )?;
    Ok(path)
}
