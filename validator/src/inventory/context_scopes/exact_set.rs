use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::{read_bounded, relative};
use crate::inventory::types::InventoryError;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const CONTEXT_ID: &str = "predecessor-contract-2026-07";
const CONTEXT_ROOT: &str = "docs/ultragoal-contract-2026-07";
const CONTENT_SET_DIGEST: &str = "4239c6d91d07120e877d540a0465dc955be7982c8e9f1880e77956fcc5d02334";
const STATUS: &str = "replaced_historical_context";
const REPLACEMENT: &str = "harness-ultragoal-successor-contract-v2";
const MAX_FILES: usize = 32;
const MAX_ENTRIES: usize = 48;
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 48 * 1024 * 1024;
const MAX_DEPTH: usize = 4;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContextRow {
    context_id: String,
    root: String,
    content_set_digest: String,
    files: Vec<FileRow>,
    authority: Authority,
    active_contract_replaced: bool,
    replacement_contract_id: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRow {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    binding: bool,
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

fn valid_relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && Path::new(value).components().count() <= MAX_DEPTH
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn declared(row: &ContextRow) -> Option<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    if row.files.is_empty() || row.files.len() > MAX_FILES {
        return None;
    }
    for file in &row.files {
        if !valid_relative(&file.path)
            || !valid_digest(&file.sha256)
            || files
                .insert(file.path.clone(), file.sha256.clone())
                .is_some()
        {
            return None;
        }
    }
    Some(files)
}

pub(super) fn registry_valid(rows: &[ContextRow]) -> bool {
    let Some(row) = rows.first().filter(|_| rows.len() == 1) else {
        return false;
    };
    let Some(files) = declared(row) else {
        return false;
    };
    let digest_input = files
        .iter()
        .map(|(path, digest)| format!("{digest}  {path}\n"))
        .collect::<String>();
    row.context_id == CONTEXT_ID
        && row.root == CONTEXT_ROOT
        && row.content_set_digest == CONTENT_SET_DIGEST
        && sha256_hex(digest_input.as_bytes()) == CONTENT_SET_DIGEST
        && !row.authority.binding
        && row.active_contract_replaced
        && row.replacement_contract_id == REPLACEMENT
        && row.status == STATUS
}

fn expected_directories(files: &BTreeSet<String>) -> BTreeSet<String> {
    let mut directories = BTreeSet::from([String::new()]);
    for file in files {
        let mut parent = Path::new(file).parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            directories.insert(path.to_string_lossy().to_string());
            parent = path.parent();
        }
    }
    directories
}

fn actual_entries(
    reads: &ReadSession,
    repository_root: &Path,
) -> Result<(BTreeSet<String>, BTreeSet<String>), InventoryError> {
    let root = repository_root.join(CONTEXT_ROOT);
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();
    for (visited, entry) in walkdir::WalkDir::new(&root)
        .follow_links(false)
        .max_depth(MAX_DEPTH + 1)
        .into_iter()
        .enumerate()
    {
        if visited >= MAX_ENTRIES {
            return Err(invalid("predecessor context exceeds its entry bound"));
        }
        reads
            .charge_entry()
            .map_err(|_| invalid("predecessor context exceeds the read-session bound"))?;
        let entry = entry.map_err(|_| invalid("predecessor context enumeration failed"))?;
        if entry.depth() > MAX_DEPTH {
            return Err(invalid("predecessor context exceeds its depth bound"));
        }
        let within = if entry.depth() == 0 {
            String::new()
        } else {
            relative(&root, entry.path())?
        };
        if entry.file_type().is_dir() {
            #[cfg(unix)]
            {
                let metadata = entry
                    .metadata()
                    .map_err(|_| invalid("predecessor directory metadata is unavailable"))?;
                let pinned = reads
                    .pin_directory(entry.path())
                    .map_err(|_| invalid("predecessor directory is not confined"))?;
                if pinned != (metadata.dev(), metadata.ino()) {
                    return Err(invalid("predecessor directory identity changed"));
                }
            }
            directories.insert(within);
        } else if entry.file_type().is_file() && !entry.file_type().is_symlink() {
            if !valid_relative(&within) || !files.insert(within) || files.len() > MAX_FILES {
                return Err(invalid("predecessor context file coverage is invalid"));
            }
        } else {
            return Err(invalid("predecessor context contains a non-regular entry"));
        }
    }
    Ok((files, directories))
}

pub(super) fn verify(
    reads: &ReadSession,
    repository_root: &Path,
    row: &ContextRow,
) -> Result<Vec<String>, InventoryError> {
    if !registry_valid(std::slice::from_ref(row)) {
        return Err(invalid("predecessor context registry row is invalid"));
    }
    let declared = declared(row).ok_or_else(|| invalid("predecessor file rows are invalid"))?;
    let expected = declared.keys().cloned().collect::<BTreeSet<_>>();
    let (actual_files, actual_directories) = actual_entries(reads, repository_root)?;
    if actual_files != expected || actual_directories != expected_directories(&expected) {
        return Err(invalid(
            "predecessor context coverage is incomplete or excessive",
        ));
    }
    let mut total = 0_u64;
    for (path, digest) in declared {
        let bytes = read_bounded(
            reads,
            &repository_root.join(CONTEXT_ROOT).join(&path),
            MAX_FILE_BYTES,
        )?;
        total = total.saturating_add(bytes.len() as u64);
        if total > MAX_TOTAL_BYTES || sha256_hex(&bytes) != digest {
            return Err(invalid("predecessor context digest verification failed"));
        }
    }
    Ok(expected
        .into_iter()
        .map(|path| format!("{CONTEXT_ROOT}/{path}"))
        .collect())
}

pub(super) const fn context_id() -> &'static str {
    CONTEXT_ID
}
