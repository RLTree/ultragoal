pub(crate) mod names;
pub(crate) mod zip;

use crate::digest;
use crate::json_boundary;
use serde_json::{Value, json};
use std::path::Path;

const JUNK_DIRS: &[&str] = &["__MACOSX", "__pycache__", ".serena"];
const JUNK_FILES: &[&str] = &[".DS_Store"];
const JUNK_SUFFIXES: &[&str] = &[".pyc", ".pyo"];

pub fn build_archive(
    root: &Path,
    zip_path: &Path,
    zip_root: &str,
    archive_purpose: &str,
) -> Result<Value, String> {
    let zip_root = crate::archive::names::validate_zip_root(zip_root)?;
    let purpose = validate_archive_purpose(archive_purpose)?;
    let manifest = json_boundary::read_json(&root.join("plugin-manifest-draft.json"))?;
    let entries = archive_paths(&manifest);
    archive_inputs_closed(root, &entries)?;
    let mut rows = read_entries(root, &zip_root, &entries)?;
    crate::archive::zip::write_zip(zip_path, &mut rows).map_err(redact_error)?;
    let entry_names = rows.iter().map(|row| row.name.clone()).collect::<Vec<_>>();
    let entry_digest = entry_list_digest(&entry_names);
    Ok(json!({
        "schema": "harness-ultragoal.distribution-archive-receipt.v1",
        "status": "pass",
        "generated_at": "1980-01-01T00:00:00Z",
        "archive_purpose": purpose,
        "claim_ceiling": archive_claim_ceiling(purpose),
        "archive": {
            "path": zip_path.to_string_lossy(),
            "digest": digest::file(zip_path)?,
            "entry_count": rows.len(),
            "root_names": [&zip_root],
            "entry_list_digest": entry_digest
        },
        "source": {
            "root": ".",
            "package_digest": crate::package::inventory::package_digest(root)?,
            "manifest_path": "plugin-manifest-draft.json",
            "manifest_digest": digest::file(&root.join("plugin-manifest-draft.json"))?
        },
        "hygiene": {
            "status": "pass",
            "excluded_junk_dirs": JUNK_DIRS,
            "excluded_junk_files": JUNK_FILES,
            "excluded_junk_suffixes": JUNK_SUFFIXES,
            "bad_entries": [],
            "symlinks": [],
            "path_traversal_entries": [],
            "duplicate_entries": []
        },
        "determinism": {
            "fixed_timestamp": "1980-01-01T00:00:00Z",
            "sorted_entries": true,
            "compression": "ZIP_STORED"
        }
    }))
}

fn validate_archive_purpose(value: &str) -> Result<&str, String> {
    match value {
        "candidate_review_anchor" => Ok(value),
        _ => Err(format!(
            "archive purpose must be candidate_review_anchor: {value}"
        )),
    }
}

fn archive_claim_ceiling(_purpose: &str) -> &'static str {
    "detached candidate review anchor only; not upload or distribution proof"
}

fn entry_list_digest(entry_names: &[String]) -> String {
    digest::bytes(entry_names.join("\n").as_bytes())
}

fn archive_paths(manifest: &Value) -> Vec<String> {
    let mut paths = crate::package::inventory::inventory_paths(manifest);
    paths.sort();
    paths.dedup();
    paths
}

fn archive_inputs_closed(root: &Path, entries: &[String]) -> Result<(), String> {
    let mut invalid = Vec::new();
    for rel in entries {
        if let Some(error) = crate::package::inventory::package_path_error(root, rel) {
            invalid.push(format!("{rel}: {error}"));
            continue;
        }
        if let Some(error) = path_hygiene_error(rel) {
            invalid.push(error);
            continue;
        }
        let path = crate::package::inventory::resolve(root, rel)?;
        if !path.exists() {
            invalid.push(format!("{rel}: manifest path does not exist"));
        } else if !path.is_file() {
            invalid.push(format!("{rel}: archive entry is not a file"));
        } else if file_contains_private_path(&path) {
            invalid.push(format!("{rel}: archive entry contains private home path"));
        }
    }
    if invalid.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "archive inputs failed hygiene: {}",
            invalid.join("; ")
        ))
    }
}

fn file_contains_private_path(path: &Path) -> bool {
    let Ok(bytes) = digest::read_file_bytes(path) else {
        return false;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return false;
    };
    text.contains(concat!("/", "Users/"))
}

fn path_hygiene_error(rel: &str) -> Option<String> {
    let parts = rel.split('/').collect::<Vec<_>>();
    if parts.iter().any(|part| JUNK_DIRS.contains(part)) {
        return Some(format!("junk directory is not archiveable: {rel}"));
    }
    if parts.last().is_some_and(|name| JUNK_FILES.contains(name)) {
        return Some(format!("junk file is not archiveable: {rel}"));
    }
    if JUNK_SUFFIXES.iter().any(|suffix| rel.ends_with(suffix)) {
        return Some(format!("bytecode/cache file is not archiveable: {rel}"));
    }
    None
}

fn read_entries(
    root: &Path,
    zip_root: &str,
    entries: &[String],
) -> Result<Vec<crate::archive::zip::Entry>, String> {
    let mut rows = Vec::new();
    for rel in entries {
        let path = crate::package::inventory::resolve(root, rel).map_err(redact_error)?;
        let bytes = digest::read_file_bytes(&path).map_err(|err| {
            format!(
                "{}: archive read failed: {}",
                redact_path(&path),
                redact_error(err)
            )
        })?;
        rows.push(crate::archive::zip::Entry {
            name: crate::archive::names::entry_name(zip_root, rel)?,
            crc32: crate::archive::zip::crc32(&bytes),
            bytes,
            offset: 0,
        });
    }
    Ok(rows)
}

fn redact_error(error: String) -> String {
    crate::cli::observe::telemetry::redact_sensitive_text(&error)
}

fn redact_path(path: &Path) -> String {
    crate::cli::observe::telemetry::redact_sensitive_text(&path.display().to_string())
}
