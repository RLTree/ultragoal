use super::CONTRACT_DIR;
use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::read_bounded;
use crate::inventory::types::InventoryError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

const MAX_BUNDLE_FILES: usize = 256;
const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CONTRACT_FILE_BYTES: u64 = 64 * 1024 * 1024;

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn safe_relative(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn declared_files(bytes: &[u8]) -> Result<BTreeMap<String, String>, InventoryError> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("handoff manifest is not UTF-8"))?;
    let mut declared = BTreeMap::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (digest, path) = line
            .split_once("  ")
            .ok_or_else(|| invalid("handoff manifest contains a malformed row"))?;
        if !valid_digest(digest) || !safe_relative(path) {
            return Err(invalid("handoff manifest contains a noncanonical row"));
        }
        if declared
            .insert(path.to_owned(), digest.to_owned())
            .is_some()
        {
            return Err(invalid("handoff manifest contains a duplicate path"));
        }
        if declared.len() > MAX_BUNDLE_FILES {
            return Err(invalid("handoff manifest exceeds the file-count limit"));
        }
    }
    if declared.is_empty() {
        return Err(invalid("handoff manifest is empty"));
    }
    Ok(declared)
}

fn entry_path(entry: fs::DirEntry) -> Result<(String, fs::FileType), InventoryError> {
    let name = entry
        .file_name()
        .into_string()
        .map_err(|_| invalid("contract bundle contains a non-UTF-8 path"))?;
    let kind = entry.file_type().map_err(|error| InventoryError::Io {
        path: entry.path(),
        message: error.to_string(),
    })?;
    Ok((name, kind))
}

fn actual_files(reads: &ReadSession, bundle: &Path) -> Result<BTreeSet<String>, InventoryError> {
    reads
        .pin_directory(bundle)
        .map_err(|error| InventoryError::Io {
            path: bundle.to_path_buf(),
            message: error.to_string(),
        })?;
    let contract = bundle.join("FINAL-CONTRACT");
    reads
        .pin_directory(&contract)
        .map_err(|error| InventoryError::Io {
            path: contract.clone(),
            message: error.to_string(),
        })?;
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(bundle).map_err(|error| InventoryError::Io {
        path: bundle.to_path_buf(),
        message: error.to_string(),
    })? {
        reads.charge_entry().map_err(|error| InventoryError::Io {
            path: bundle.to_path_buf(),
            message: error.to_string(),
        })?;
        let entry = entry.map_err(|error| InventoryError::Io {
            path: bundle.to_path_buf(),
            message: error.to_string(),
        })?;
        let (name, kind) = entry_path(entry)?;
        if name == "FINAL-HANDOFF-MANIFEST.sha256" && kind.is_file() {
            continue;
        }
        if name == "FINAL-CONTRACT" && kind.is_dir() {
            continue;
        }
        if !kind.is_file() || !safe_relative(&name) {
            return Err(invalid(
                "contract bundle contains an unsupported root entry",
            ));
        }
        actual.insert(name);
    }
    for entry in fs::read_dir(&contract).map_err(|error| InventoryError::Io {
        path: contract.clone(),
        message: error.to_string(),
    })? {
        reads.charge_entry().map_err(|error| InventoryError::Io {
            path: contract.clone(),
            message: error.to_string(),
        })?;
        let entry = entry.map_err(|error| InventoryError::Io {
            path: contract.clone(),
            message: error.to_string(),
        })?;
        let (name, kind) = entry_path(entry)?;
        if !kind.is_file() || !safe_relative(&name) {
            return Err(invalid("FINAL-CONTRACT contains an unsupported entry"));
        }
        actual.insert(format!("FINAL-CONTRACT/{name}"));
        if actual.len() > MAX_BUNDLE_FILES {
            return Err(invalid("contract bundle exceeds the file-count limit"));
        }
    }
    Ok(actual)
}

fn verify_contract_manifest(
    manifest_bytes: &[u8],
    observed: &BTreeMap<String, (String, usize)>,
) -> Result<(), InventoryError> {
    let value: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|_| invalid("CONTRACT_MANIFEST.json is invalid JSON"))?;
    let rows = value
        .get("contract_entries")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("CONTRACT_MANIFEST.json lacks contract_entries"))?;
    let expected_paths = observed
        .keys()
        .filter(|path| {
            path.starts_with("FINAL-CONTRACT/")
                && path.as_str() != "FINAL-CONTRACT/CONTRACT_MANIFEST.json"
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut declared = BTreeSet::new();
    for row in rows {
        let path = row
            .get("path")
            .and_then(Value::as_str)
            .filter(|path| safe_relative(path))
            .ok_or_else(|| invalid("contract entry has an invalid path"))?;
        let digest = row
            .get("sha256")
            .and_then(Value::as_str)
            .filter(|digest| valid_digest(digest))
            .ok_or_else(|| invalid("contract entry has an invalid digest"))?;
        let bytes = row
            .get("bytes")
            .and_then(Value::as_u64)
            .and_then(|bytes| usize::try_from(bytes).ok())
            .ok_or_else(|| invalid("contract entry has an invalid byte length"))?;
        if observed.get(path) != Some(&(digest.to_owned(), bytes))
            || !declared.insert(path.to_owned())
        {
            return Err(invalid("contract entry disagrees with its adopted file"));
        }
    }
    if declared != expected_paths {
        return Err(invalid(
            "contract_entries coverage is incomplete or excessive",
        ));
    }
    Ok(())
}

pub(super) fn verify(
    reads: &ReadSession,
    root: &Path,
    expected_manifest_digest: &str,
) -> Result<(), InventoryError> {
    if !valid_digest(expected_manifest_digest) {
        return Err(invalid("root-bound handoff digest is not canonical"));
    }
    let contract = root.join(CONTRACT_DIR);
    let bundle = contract
        .parent()
        .ok_or_else(|| invalid("contract bundle has no parent"))?;
    let manifest_path = bundle.join("FINAL-HANDOFF-MANIFEST.sha256");
    let manifest_bytes = read_bounded(reads, &manifest_path, MAX_MANIFEST_BYTES)?;
    if sha256_hex(&manifest_bytes) != expected_manifest_digest {
        return Err(invalid(
            "handoff manifest disagrees with the root-bound digest",
        ));
    }
    let declared = declared_files(&manifest_bytes)?;
    if declared.keys().cloned().collect::<BTreeSet<_>>() != actual_files(reads, bundle)? {
        return Err(invalid(
            "handoff manifest coverage is incomplete or excessive",
        ));
    }
    let mut observed = BTreeMap::new();
    for (relative, expected) in declared {
        let bytes = read_bounded(reads, &bundle.join(&relative), MAX_CONTRACT_FILE_BYTES)?;
        let digest = sha256_hex(&bytes);
        if digest != expected {
            return Err(invalid(
                "handoff file digest does not match the adopted manifest",
            ));
        }
        observed.insert(relative, (digest, bytes.len()));
    }
    let contract_manifest = observed
        .get("FINAL-CONTRACT/CONTRACT_MANIFEST.json")
        .ok_or_else(|| invalid("handoff manifest omits CONTRACT_MANIFEST.json"))?;
    let bytes = read_bounded(
        reads,
        &bundle.join("FINAL-CONTRACT/CONTRACT_MANIFEST.json"),
        MAX_MANIFEST_BYTES,
    )?;
    if (sha256_hex(&bytes), bytes.len()) != *contract_manifest {
        return Err(invalid(
            "CONTRACT_MANIFEST.json changed during verification",
        ));
    }
    verify_contract_manifest(&bytes, &observed)
}
