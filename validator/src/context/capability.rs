use super::bound_context::{CapabilitySet, ToolCapability};
use super::digest::sha256_hex;
use super::error::{ContextError, io_error};
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const DEFAULT_GIT_PATHS: &[&str] = &["/usr/bin/git", "/bin/git"];

fn os_bytes(value: &OsStr) -> Vec<u8> {
    #[cfg(unix)]
    {
        value.as_bytes().to_vec()
    }
    #[cfg(not(unix))]
    {
        value.to_string_lossy().as_bytes().to_vec()
    }
}

fn valid_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(&byte))
}

fn executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn canonical_executable(path: &Path) -> Result<PathBuf, ContextError> {
    let canonical = path.canonicalize().map_err(|error| io_error(path, error))?;
    if !executable(&canonical) {
        return Err(ContextError::InvalidRequest(format!(
            "Git substrate is not an executable regular file: {}",
            canonical.display()
        )));
    }
    if canonical.to_str().is_none() {
        return Err(ContextError::InvalidRequest(format!(
            "Git substrate path is not UTF-8: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

pub(crate) fn resolve_git() -> Result<PathBuf, ContextError> {
    for path in DEFAULT_GIT_PATHS {
        let path = Path::new(path);
        if executable(path) {
            return canonical_executable(path);
        }
    }
    Err(ContextError::UnsupportedCapability(
        "no fixed protected Git substrate at /usr/bin/git or /bin/git".to_owned(),
    ))
}

fn find_on_path(name: &str, path: &OsStr) -> Result<Option<PathBuf>, ContextError> {
    if !valid_tool_name(name) {
        return Err(ContextError::InvalidRequest(format!(
            "unsafe tool discovery name {name:?}"
        )));
    }
    for directory in env::split_paths(path) {
        let candidate = directory.join(name);
        if executable(&candidate) {
            let canonical = candidate
                .canonicalize()
                .map_err(|error| io_error(candidate, error))?;
            if canonical.to_str().is_none() {
                return Err(ContextError::InvalidRequest(format!(
                    "tool path is not UTF-8: {}",
                    canonical.display()
                )));
            }
            return Ok(Some(canonical));
        }
    }
    Ok(None)
}

fn file_digest(path: &Path) -> Result<(String, u64), ContextError> {
    let mut file = File::open(path).map_err(|error| io_error(path, error))?;
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| io_error(path, error))?;
        if read == 0 {
            break;
        }
        length += read as u64;
        hasher.update(&buffer[..read]);
    }
    Ok((format!("{:x}", hasher.finalize()), length))
}

pub(crate) fn executable_identity(name: &str, path: &Path) -> Result<ToolCapability, ContextError> {
    let canonical = canonical_executable(path)?;
    let metadata = fs::metadata(&canonical).map_err(|error| io_error(&canonical, error))?;
    let (executable_sha256, byte_length) = file_digest(&canonical)?;
    #[cfg(unix)]
    let unix_mode = Some(metadata.permissions().mode());
    #[cfg(not(unix))]
    let unix_mode = None;
    Ok(ToolCapability {
        name: name.to_owned(),
        available: true,
        executable: Some(canonical.to_str().expect("validated UTF-8").to_owned()),
        executable_sha256: Some(executable_sha256),
        byte_length: Some(byte_length),
        unix_mode,
    })
}

fn missing(name: String) -> ToolCapability {
    ToolCapability {
        name,
        available: false,
        executable: None,
        executable_sha256: None,
        byte_length: None,
        unix_mode: None,
    }
}

pub(crate) fn capture(names: &[String], git: &Path) -> Result<CapabilitySet, ContextError> {
    let path = env::var_os("PATH").unwrap_or_default();
    let mut names = names.to_vec();
    names.sort();
    names.dedup();
    let mut tools = Vec::with_capacity(names.len());
    for name in names {
        let discovered = if name == "git" {
            Some(git.to_path_buf())
        } else {
            find_on_path(&name, &path)?
        };
        tools.push(match discovered {
            Some(executable) => executable_identity(&name, &executable)?,
            None => missing(name),
        });
    }
    Ok(CapabilitySet {
        path_search_sha256: sha256_hex(&os_bytes(&path)),
        tools,
    })
}
