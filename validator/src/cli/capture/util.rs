use crate::context::{CandidateIdentity, LiveContext};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::path::{Component, Path};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(super) fn candidate_id(candidate: &CandidateIdentity) -> Result<String, String> {
    serde_json::to_vec(candidate)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| "candidate identity serialization failed".to_owned())
}

pub(super) fn context_candidate_id(context: &LiveContext) -> Result<String, String> {
    candidate_id(context.candidate())
}

pub(super) fn bytes_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(unix)]
pub(super) fn os_bytes(value: &OsStr) -> &[u8] {
    value.as_bytes()
}

#[cfg(not(unix))]
pub(super) fn os_bytes(value: &OsStr) -> &[u8] {
    value.to_str().unwrap_or("").as_bytes()
}

pub(super) fn validate_relative(path: &Path, label: &str) -> Result<(), String> {
    if path.is_absolute() {
        return Err(format!("{label} must be worktree-relative"));
    }
    let mut normal = 0_usize;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => normal += 1,
            _ => return Err(format!("{label} contains a forbidden path component")),
        }
    }
    if normal == 0 && path != Path::new(".") {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

pub(super) fn validate_public_path(path: &Path, label: &str) -> Result<(), String> {
    if path.components().any(
        |component| matches!(component, Component::Normal(value) if is_prohibited_component(value)),
    ) {
        return Err(format!(
            "{label} enters a prohibited secret-bearing worktree path"
        ));
    }
    Ok(())
}

pub(super) fn is_prohibited_component(value: &OsStr) -> bool {
    const PROHIBITED: &[u8] = b".codex-worktree";
    #[cfg(unix)]
    {
        value.as_bytes().eq_ignore_ascii_case(PROHIBITED)
    }
    #[cfg(not(unix))]
    {
        value
            .to_str()
            .is_some_and(|text| text.as_bytes().eq_ignore_ascii_case(PROHIBITED))
    }
}

pub(super) fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|part| part == needle)
}
