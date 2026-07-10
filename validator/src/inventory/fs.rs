use super::digest::file_identity;
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_TEXT_METADATA_BYTES: u64 = 1024 * 1024;

pub(crate) fn read_bounded(
    reads: &ReadSession,
    path: &Path,
    maximum_bytes: u64,
) -> Result<Vec<u8>, InventoryError> {
    reads
        .read_bounded(path, maximum_bytes)
        .map_err(|error| InventoryError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        })
}

pub(crate) fn relative(root: &Path, path: &Path) -> Result<String, InventoryError> {
    path.strip_prefix(root)
        .map_err(|_| InventoryError::PathEscape(path.to_path_buf()))?
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            InventoryError::InvalidRegistry(format!("non-UTF-8 path: {}", path.display()))
        })
}

pub(crate) fn check_symlink(
    root: &Path,
    path: &Path,
    findings: &mut Vec<InventoryFinding>,
) -> Result<bool, InventoryError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| InventoryError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !metadata.file_type().is_symlink() {
        return Ok(true);
    }
    let resolved = path.canonicalize().map_err(|error| InventoryError::Io {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if !resolved.starts_with(root) {
        let rel = relative(root, path)?;
        findings.push(InventoryFinding::error(
            "symlink_path_escape",
            None,
            Some(&rel),
            "symlink resolves outside the worktree".to_owned(),
        ));
        return Ok(false);
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn physical_entry(
    reads: &ReadSession,
    root: &Path,
    path: &Path,
    stable_id: String,
    kind: &str,
    owner: &str,
    authority_state: AuthorityState,
    active_status: ActiveStatus,
    generator: Option<String>,
    provenance: Vec<String>,
    references: Vec<String>,
) -> Result<InventoryEntry, InventoryError> {
    let (digest_sha256, unix_mode) = file_identity(reads, path)?;
    Ok(InventoryEntry {
        stable_id,
        kind: kind.to_owned(),
        owner_role: owner.to_owned(),
        relative_path: relative(root, path)?,
        digest_sha256,
        unix_mode,
        authority_state,
        active_status,
        generator,
        input_provenance: provenance,
        references,
    })
}

pub(crate) fn contract_source_entry(
    reads: &ReadSession,
    root: &Path,
    path: &Path,
    name: &str,
) -> Result<InventoryEntry, InventoryError> {
    physical_entry(
        reads,
        root,
        path,
        format!("CONTRACT-REGISTRY:{name}"),
        "contract-registry",
        "OWN-ULTRA-ROOT",
        AuthorityState::Canonical,
        ActiveStatus::Active,
        None,
        Vec::new(),
        Vec::new(),
    )
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn component_name(
    reads: &ReadSession,
    path: &Path,
    require_frontmatter: bool,
) -> Option<String> {
    let bytes = read_bounded(reads, path, MAX_TEXT_METADATA_BYTES).ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    let lines = text.lines().take(40).collect::<Vec<_>>();
    let end = if require_frontmatter {
        (lines.first()?.trim() == "---")
            .then_some(())
            .and_then(|_| lines.iter().skip(1).position(|line| line.trim() == "---"))?
            + 1
    } else {
        lines.len()
    };
    let fields = if require_frontmatter {
        &lines[1..end]
    } else {
        &lines[..end]
    };
    let mut name = None;
    let mut description = false;
    for line in fields {
        let trimmed = line.trim();
        for separator in [':', '='] {
            let Some((key, value)) = trimmed
                .split_once(separator)
                .filter(|(key, _)| matches!(key.trim(), "name" | "description"))
            else {
                continue;
            };
            let value = value.trim().trim_matches(['"', '\'']);
            if key.trim() == "name" {
                name = safe_identifier(value).then(|| value.to_owned());
            } else if !value.is_empty()
                && value.len() <= 1024
                && !value.bytes().any(|byte| byte.is_ascii_control())
            {
                description = true;
            }
        }
    }
    description.then_some(name).flatten()
}

pub(crate) fn regular_files(
    reads: &ReadSession,
    root: &Path,
    relative_root: &str,
) -> Result<Vec<PathBuf>, InventoryError> {
    super::walk::collection_files(reads, &root.join(relative_root))
}
