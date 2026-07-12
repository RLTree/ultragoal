use crate::context::ReadSession;
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::{read_bounded, relative};
use crate::inventory::types::InventoryError;
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const CONTEXT_ID: &str = "successor-worker-results";
const CONTEXT_ROOT: &str = "docs/ultragoal-successor-live/worker-results";
const PATH_POLICY: &str = "lease-json-worker-result-v1";
const SCHEMA_REF: &str = "docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/IMPLEMENTATION_DEPENDENCY_GRAPH.json#/worker_output_schema";
const STATUS: &str = "run_scoped_evidence_only";
const NO_CLAIM: &str = "This worker does not claim readiness, release, or completion.";
const HISTORICAL_CONTENT_SET_DIGEST: &str =
    "0055b9045e3631cad124e4e194985af11137a47d597f60657255e432b53e2f8a";
const MAX_FILES: usize = 256;
const MAX_HISTORICAL_FILES: usize = 16;
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARRAY_ITEMS: usize = 4096;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContextRow {
    context_id: String,
    root: String,
    path_policy: String,
    schema_ref: String,
    historical_content_set_digest: String,
    historical_files: Vec<HistoricalFile>,
    authority: Authority,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalFile {
    path: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    binding: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerResult {
    worker: String,
    lease_id: String,
    context_id: String,
    candidate_identity: Map<String, Value>,
    base_state: Map<String, Value>,
    final_state: Map<String, Value>,
    touched_paths: Vec<String>,
    touched_semantics: Vec<String>,
    generated_outputs: Vec<String>,
    fixtures: Vec<String>,
    effects: Vec<Map<String, Value>>,
    requirements: Vec<String>,
    dependency_nodes: Vec<String>,
    changes: Vec<Map<String, Value>>,
    commands_and_tests: Vec<Map<String, Value>>,
    artifacts: Vec<Map<String, Value>>,
    findings: Vec<Map<String, Value>>,
    unresolved_dependencies: Vec<String>,
    requested_root_changes: Vec<Map<String, Value>>,
    limitations: Vec<String>,
    no_claim_statement: String,
}

fn invalid(message: &str) -> InventoryError {
    InventoryError::InvalidRegistry(message.to_owned())
}

pub(super) fn registry_valid(rows: &[ContextRow]) -> bool {
    rows.len() == 1
        && rows.first().is_some_and(|row| {
            let historical = declared_historical(row);
            row.context_id == CONTEXT_ID
                && row.root == CONTEXT_ROOT
                && row.path_policy == PATH_POLICY
                && row.schema_ref == SCHEMA_REF
                && row.historical_content_set_digest == HISTORICAL_CONTENT_SET_DIGEST
                && historical.is_some_and(|files| {
                    let digest_input = files
                        .iter()
                        .map(|(path, digest)| format!("{digest}  {path}\n"))
                        .collect::<String>();
                    sha256_hex(digest_input.as_bytes()) == HISTORICAL_CONTENT_SET_DIGEST
                })
                && !row.authority.binding
                && row.status == STATUS
        })
}

fn valid_name(name: &str) -> bool {
    name.starts_with("LEASE-")
        && name.ends_with(".json")
        && name.len() <= 192
        && name.bytes().all(|byte| {
            byte.is_ascii_uppercase()
                || byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn declared_historical(row: &ContextRow) -> Option<BTreeMap<String, String>> {
    if row.historical_files.is_empty() || row.historical_files.len() > MAX_HISTORICAL_FILES {
        return None;
    }
    let mut files = BTreeMap::new();
    for file in &row.historical_files {
        if !valid_name(&file.path)
            || file.path.contains('/')
            || !hex_digest(&file.sha256)
            || files
                .insert(file.path.clone(), file.sha256.clone())
                .is_some()
        {
            return None;
        }
    }
    Some(files)
}

fn unique_bounded(values: &[String]) -> bool {
    values.len() <= MAX_ARRAY_ITEMS && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn objects_bounded(values: &[Map<String, Value>]) -> bool {
    values.len() <= MAX_ARRAY_ITEMS
}

fn valid_record(bytes: &[u8], file_name: &str) -> bool {
    if !crate::inventory::plugin_manifest_json::unique_keys(bytes) {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<WorkerResult>(bytes) else {
        return false;
    };
    let expected_lease = file_name.strip_suffix(".json").unwrap_or_default();
    value.lease_id == expected_lease
        && !value.worker.trim().is_empty()
        && value.worker.len() <= 256
        && !value.context_id.trim().is_empty()
        && value.context_id.len() <= 256
        && value.no_claim_statement == NO_CLAIM
        && unique_bounded(&value.touched_paths)
        && unique_bounded(&value.touched_semantics)
        && unique_bounded(&value.generated_outputs)
        && unique_bounded(&value.fixtures)
        && unique_bounded(&value.requirements)
        && unique_bounded(&value.dependency_nodes)
        && unique_bounded(&value.unresolved_dependencies)
        && value.limitations.len() <= MAX_ARRAY_ITEMS
        && objects_bounded(&value.effects)
        && objects_bounded(&value.changes)
        && objects_bounded(&value.commands_and_tests)
        && objects_bounded(&value.artifacts)
        && objects_bounded(&value.findings)
        && objects_bounded(&value.requested_root_changes)
        && value.candidate_identity.len() <= MAX_ARRAY_ITEMS
        && value.base_state.len() <= MAX_ARRAY_ITEMS
        && value.final_state.len() <= MAX_ARRAY_ITEMS
}

pub(super) fn verify(
    reads: &ReadSession,
    repository_root: &Path,
    row: &ContextRow,
) -> Result<Vec<String>, InventoryError> {
    if !registry_valid(std::slice::from_ref(row)) {
        return Err(invalid("worker evidence context registry row is invalid"));
    }
    let historical = declared_historical(row)
        .ok_or_else(|| invalid("historical worker evidence rows are invalid"))?;
    let root = repository_root.join(CONTEXT_ROOT);
    let metadata = fs::symlink_metadata(&root).map_err(|_| {
        invalid("worker evidence directory is unavailable or not a regular directory")
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid("worker evidence directory is not confined"));
    }
    #[cfg(unix)]
    {
        let pinned = reads
            .pin_directory(&root)
            .map_err(|_| invalid("worker evidence directory is not descriptor-pinned"))?;
        if pinned != (metadata.dev(), metadata.ino()) {
            return Err(invalid("worker evidence directory identity changed"));
        }
    }
    let entries =
        fs::read_dir(&root).map_err(|_| invalid("worker evidence directory enumeration failed"))?;
    let mut files = BTreeSet::new();
    let mut historical_seen = BTreeSet::new();
    let mut total = 0_u64;
    for entry in entries {
        reads
            .charge_entry()
            .map_err(|_| invalid("worker evidence exceeds the read-session bound"))?;
        let entry = entry.map_err(|_| invalid("worker evidence enumeration failed"))?;
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid("worker evidence filename is not UTF-8"))?;
        if !valid_name(&file_name) {
            continue;
        }
        let file_type = entry
            .file_type()
            .map_err(|_| invalid("worker evidence file type is unavailable"))?;
        if !file_type.is_file() || file_type.is_symlink() {
            return Err(invalid(
                "worker evidence policy matched a non-regular entry",
            ));
        }
        let bytes = read_bounded(reads, &entry.path(), MAX_FILE_BYTES)?;
        total = total.saturating_add(bytes.len() as u64);
        let record_valid = match historical.get(&file_name) {
            Some(expected) => {
                let exact = sha256_hex(&bytes) == *expected;
                if exact {
                    historical_seen.insert(file_name.clone());
                }
                exact
            }
            None => valid_record(&bytes, &file_name),
        };
        if total > MAX_TOTAL_BYTES || !record_valid {
            return Err(invalid("worker evidence record is malformed or mismatched"));
        }
        let relative = relative(repository_root, &entry.path())?;
        if !files.insert(relative) || files.len() > MAX_FILES {
            return Err(invalid("worker evidence coverage is invalid"));
        }
    }
    if historical_seen != historical.keys().cloned().collect() {
        return Err(invalid("historical worker evidence coverage is incomplete"));
    }
    Ok(files.into_iter().collect())
}

pub(super) const fn context_id() -> &'static str {
    CONTEXT_ID
}

pub(super) const fn schema_ref() -> &'static str {
    SCHEMA_REF
}
