use super::{
    CANDIDATE_CONTRACT_ID, CANDIDATE_ROOT, CANDIDATE_STATUS, CONTENT_SET_SHA256,
    CONTRACT_MANIFEST_SHA256, ZIP_MANIFEST_SHA256,
};
use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::{read_bounded, relative};
use crate::inventory::types::InventoryError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const ZIP_MANIFEST: &str = "ZIP_INCLUDE_MANIFEST.json";
const CONTRACT_MANIFEST: &str = "CONTRACT_MANIFEST.json";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_FILES: usize = 64;
const MAX_ENTRIES: usize = 96;
const MAX_PATH_DEPTH: usize = 8;
const EXPECTED_LISTED_FILES: usize = 17;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ZipManifest {
    schema_version: String,
    archive_root: String,
    include_policy: String,
    digest_algorithm: String,
    digest_input_format: String,
    content_set_digest: String,
    self_digest_included: bool,
    files: Vec<FileRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRow {
    path: String,
    sha256: String,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && Path::new(value).components().count() <= MAX_PATH_DEPTH
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

fn parse_zip(bytes: &[u8]) -> Result<ZipManifest, InventoryError> {
    if sha256_hex(bytes) != ZIP_MANIFEST_SHA256
        || !crate::inventory::plugin_manifest_json::unique_keys(bytes)
    {
        return Err(invalid("candidate ZIP manifest identity is invalid"));
    }
    let parsed: ZipManifest = serde_json::from_slice(bytes)
        .map_err(|_| invalid("candidate ZIP manifest shape is invalid"))?;
    if parsed.schema_version != "1.0.0-candidate"
        || parsed.archive_root != "ultragoal-contract-2026-07-successor-candidate-v1"
        || parsed.include_policy.is_empty()
        || parsed.include_policy.len() > 4096
        || parsed.digest_algorithm != "sha256"
        || parsed.digest_input_format.is_empty()
        || parsed.digest_input_format.len() > 4096
        || parsed.content_set_digest != CONTENT_SET_SHA256
        || parsed.self_digest_included
        || parsed.files.len() != EXPECTED_LISTED_FILES
        || parsed.files.len() > MAX_FILES
    {
        return Err(invalid("candidate ZIP manifest semantics are invalid"));
    }
    Ok(parsed)
}

fn declared_files(manifest: ZipManifest) -> Result<BTreeMap<String, String>, InventoryError> {
    let mut files = BTreeMap::new();
    for row in manifest.files {
        if !valid_relative(&row.path)
            || !valid_digest(&row.sha256)
            || files.insert(row.path, row.sha256).is_some()
        {
            return Err(invalid("candidate ZIP manifest file row is invalid"));
        }
    }
    Ok(files)
}

struct ActualEntries {
    files: BTreeSet<String>,
    directories: BTreeSet<String>,
}

fn expected_directories(files: &BTreeSet<String>) -> BTreeSet<String> {
    let mut directories = BTreeSet::from([String::new()]);
    for file in files {
        let mut parent = Path::new(file).parent();
        while let Some(path) = parent.filter(|path| !path.as_os_str().is_empty()) {
            directories.insert(
                path.to_str()
                    .expect("manifest paths are valid UTF-8")
                    .to_owned(),
            );
            parent = path.parent();
        }
    }
    directories
}

fn actual_entries(reads: &ReadSession, root: &Path) -> Result<ActualEntries, InventoryError> {
    let candidate = root.join(CANDIDATE_ROOT);
    let mut files = BTreeSet::new();
    let mut directories = BTreeSet::new();
    for (visited, entry) in walkdir::WalkDir::new(&candidate)
        .follow_links(false)
        .max_depth(MAX_PATH_DEPTH + 1)
        .into_iter()
        .enumerate()
    {
        if visited >= MAX_ENTRIES {
            return Err(invalid("candidate bundle exceeds its entry-count bound"));
        }
        reads
            .charge_entry()
            .map_err(|_| invalid("candidate bundle exceeds the read-session bound"))?;
        let entry = entry.map_err(|_| invalid("candidate bundle enumeration failed"))?;
        if entry.depth() > MAX_PATH_DEPTH {
            return Err(invalid("candidate bundle exceeds its path-depth bound"));
        }
        if entry.file_type().is_dir() {
            #[cfg(unix)]
            {
                let metadata = entry
                    .metadata()
                    .map_err(|_| invalid("candidate directory metadata is unavailable"))?;
                let pinned = reads
                    .pin_directory(entry.path())
                    .map_err(|_| invalid("candidate directory is not confined"))?;
                if pinned != (metadata.dev(), metadata.ino()) {
                    return Err(invalid("candidate directory identity changed"));
                }
            }
            let within = if entry.depth() == 0 {
                String::new()
            } else {
                let full_relative = relative(root, entry.path())?;
                full_relative
                    .strip_prefix(&format!("{CANDIDATE_ROOT}/"))
                    .filter(|value| valid_relative(value))
                    .ok_or_else(|| invalid("candidate bundle contains an invalid directory"))?
                    .to_owned()
            };
            if !directories.insert(within) {
                return Err(invalid("candidate bundle directory coverage is invalid"));
            }
            continue;
        }
        if !entry.file_type().is_file() && !entry.file_type().is_symlink() {
            return Err(invalid(
                "candidate bundle contains a special filesystem entry",
            ));
        }
        let path = entry.path();
        let full_relative = relative(root, path)?;
        let within = full_relative
            .strip_prefix(&format!("{CANDIDATE_ROOT}/"))
            .filter(|value| valid_relative(value))
            .ok_or_else(|| invalid("candidate bundle contains an invalid path"))?;
        if !files.insert(within.to_owned()) || files.len() > MAX_FILES {
            return Err(invalid("candidate bundle file coverage is invalid"));
        }
    }
    Ok(ActualEntries { files, directories })
}

fn verify_contract_manifest(bytes: &[u8]) -> Result<(), InventoryError> {
    if sha256_hex(bytes) != CONTRACT_MANIFEST_SHA256
        || !crate::inventory::plugin_manifest_json::unique_keys(bytes)
    {
        return Err(invalid("candidate contract manifest identity is invalid"));
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| invalid("candidate contract manifest is invalid JSON"))?;
    if value.get("contract_id").and_then(Value::as_str) != Some(CANDIDATE_CONTRACT_ID)
        || value.get("status").and_then(Value::as_str) != Some(CANDIDATE_STATUS)
        || value
            .get("active_contract_replaced")
            .and_then(Value::as_bool)
            != Some(false)
        || value
            .get("authority")
            .and_then(|authority| authority.get("binding"))
            .and_then(Value::as_bool)
            != Some(false)
    {
        return Err(invalid("candidate contract manifest semantics are invalid"));
    }
    Ok(())
}

pub(super) fn verify(reads: &ReadSession, root: &Path) -> Result<Vec<String>, InventoryError> {
    let candidate = root.join(CANDIDATE_ROOT);
    let manifest_bytes = read_bounded(reads, &candidate.join(ZIP_MANIFEST), MAX_MANIFEST_BYTES)?;
    let declared = declared_files(parse_zip(&manifest_bytes)?)?;
    let mut expected = declared.keys().cloned().collect::<BTreeSet<_>>();
    expected.insert(ZIP_MANIFEST.to_owned());
    let expected_directories = expected_directories(&expected);
    let actual = actual_entries(reads, root)?;
    if actual.files != expected || actual.directories != expected_directories {
        return Err(invalid(
            "candidate bundle coverage is incomplete or excessive",
        ));
    }
    let mut digest_input = Vec::new();
    let mut total = 0_u64;
    for (path, digest) in &declared {
        let bytes = read_bounded(reads, &candidate.join(path), MAX_FILE_BYTES)?;
        total = total.saturating_add(bytes.len() as u64);
        if total > MAX_TOTAL_BYTES || sha256_hex(&bytes) != *digest {
            return Err(invalid("candidate bundle file digest is invalid"));
        }
        digest_input.extend_from_slice(format!("{digest}  {path}\n").as_bytes());
        if path == CONTRACT_MANIFEST {
            verify_contract_manifest(&bytes)?;
        }
    }
    if !declared.contains_key(CONTRACT_MANIFEST) || sha256_hex(&digest_input) != CONTENT_SET_SHA256
    {
        return Err(invalid("candidate bundle content-set identity is invalid"));
    }
    Ok(expected.into_iter().collect())
}
