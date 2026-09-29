use crate::distribution::HostCommand;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(super) const SANDBOX_EXECUTABLE: &str = "/usr/bin/sandbox-exec";
const SANDBOX_PROFILE: &str =
    "(version 1) (allow default) (deny process-fork (with send-signal SIGKILL))";
const READ_ONLY_SANDBOX_PROFILE: &str = "(version 1) (allow default) (deny file-write*) (deny file-clone file-link) (deny network*) (deny process-fork (with send-signal SIGKILL))";

pub(super) fn verified_executable() -> Result<CString, ()> {
    let path = Path::new(SANDBOX_EXECUTABLE);
    let metadata = std::fs::symlink_metadata(path).map_err(|_| ())?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != 0
        || metadata.mode() & 0o022 != 0
    {
        return Err(());
    }
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let file = options.open(path).map_err(|_| ())?;
    let opened = file.metadata().map_err(|_| ())?;
    if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
        return Err(());
    }
    CString::new(SANDBOX_EXECUTABLE).map_err(|_| ())
}

pub(super) fn arguments(path: &Path, command: &HostCommand) -> Result<Vec<CString>, ()> {
    let mut arguments = Vec::with_capacity(command.argv().len() + 5);
    arguments.push(CString::new(SANDBOX_EXECUTABLE).map_err(|_| ())?);
    arguments.push(CString::new("-p").map_err(|_| ())?);
    arguments.push(CString::new(SANDBOX_PROFILE).map_err(|_| ())?);
    arguments.push(CString::new(path.as_os_str().as_bytes()).map_err(|_| ())?);
    arguments.push(CString::new(command.program()).map_err(|_| ())?);
    for argument in command.argv() {
        arguments.push(CString::new(argument.as_bytes()).map_err(|_| ())?);
    }
    Ok(arguments)
}

pub(super) fn runtime_arguments(path: &Path, command: &HostCommand) -> Result<Vec<CString>, ()> {
    let mut arguments = Vec::with_capacity(command.argv().len() + 4);
    arguments.push(CString::new(SANDBOX_EXECUTABLE).map_err(|_| ())?);
    arguments.push(CString::new("-p").map_err(|_| ())?);
    arguments.push(CString::new(READ_ONLY_SANDBOX_PROFILE).map_err(|_| ())?);
    arguments.push(CString::new(path.as_os_str().as_bytes()).map_err(|_| ())?);
    for argument in command.argv() {
        arguments.push(CString::new(argument.as_bytes()).map_err(|_| ())?);
    }
    Ok(arguments)
}

pub(super) fn personal_arguments(
    path: &Path,
    command: &HostCommand,
    codex_home: &Path,
) -> Result<Vec<CString>, ()> {
    if !codex_home.is_absolute() {
        return Err(());
    }
    let codex_home = codex_home.as_os_str().as_bytes();
    if codex_home.contains(&0) {
        return Err(());
    }
    let escaped = codex_home
        .iter()
        .try_fold(String::new(), |mut output, byte| {
            match *byte {
                b'\\' => output.push_str("\\\\"),
                b'\"' => output.push_str("\\\""),
                0x20..=0x7e => output.push(*byte as char),
                _ => return Err(()),
            }
            Ok(output)
        })?;
    let profile = CString::new(format!(
        "(version 1) (allow default) (deny process-fork (with send-signal SIGKILL)) (deny file-write*) (allow file-write* (subpath \"{escaped}\"))"
    ))
    .map_err(|_| ())?;
    let mut arguments = Vec::with_capacity(command.argv().len() + 5);
    arguments.push(CString::new(SANDBOX_EXECUTABLE).map_err(|_| ())?);
    arguments.push(CString::new("-p").map_err(|_| ())?);
    arguments.push(profile);
    arguments.push(CString::new(path.as_os_str().as_bytes()).map_err(|_| ())?);
    arguments.push(CString::new(command.program()).map_err(|_| ())?);
    for argument in command.argv() {
        arguments.push(CString::new(argument.as_bytes()).map_err(|_| ())?);
    }
    Ok(arguments)
}
