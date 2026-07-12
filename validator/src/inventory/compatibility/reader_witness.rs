use super::agent_specs::{READER_PROOF_PATH, READER_PROOF_SHA256};
use super::reader_witness_specs::{ArtifactSpec, PHASE_A_ARTIFACTS};
use crate::context::ReadSession;
use crate::inventory::agent_reader_guard_digests::{MANIFESTS, ReaderSpec, all_readers};
use crate::inventory::digest::sha256_hex;
use crate::inventory::fs::{read_bounded, relative};
use crate::inventory::walk::{CollectionEntryKind, collection_entries};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Component, Path};

mod scan;
use scan::{contains_legacy_tokens, no_unbound_reader};
#[cfg(test)]
use scan::{looks_like_agent_reader, unbound_reader_paths};

const MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;
const MANIFEST_PATH: &str = "plugin-manifest-draft.json";
const GUARD_PATH: &str = "validator/src/inventory/agent_reader_guard_digests.rs";
const GUARD_BYTES: &[u8] = include_bytes!("../agent_reader_guard_digests.rs");

fn read_current(reads: &ReadSession, root: &Path, spec: ArtifactSpec) -> Option<Vec<u8>> {
    let bytes = read_bounded(reads, &root.join(spec.path), MAX_ARTIFACT_BYTES).ok()?;
    (sha256_hex(&bytes) == spec.sha256).then_some(bytes)
}

fn receipt_matches(bytes: &[u8]) -> bool {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return false;
    };
    if value.get("lease_id").and_then(Value::as_str) != Some("LEASE-N02-AGENT-READERS-002")
        || value
            .pointer("/final_state/active_leased_legacy_identity_readers")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return false;
    }
    let Some(rows) = value.get("artifacts").and_then(Value::as_array) else {
        return false;
    };
    rows.len() == PHASE_A_ARTIFACTS.len()
        && PHASE_A_ARTIFACTS.iter().all(|spec| {
            rows.iter()
                .filter(|row| {
                    row.get("path").and_then(Value::as_str) == Some(spec.path)
                        && row.get("sha256").and_then(Value::as_str) == Some(spec.sha256)
                })
                .count()
                == 1
        })
}

fn phase_a_context_intact(reads: &ReadSession, root: &Path) -> bool {
    let receipt = ArtifactSpec {
        path: READER_PROOF_PATH,
        sha256: READER_PROOF_SHA256,
    };
    // Phase A is accepted historical context, not current reader proof. Its
    // unchanged receipt must still describe the exact frozen artifact set,
    // while `guard_current` and the repository-wide scan below bind every
    // current production reader after that set changes.
    read_current(reads, root, receipt).is_some_and(|bytes| receipt_matches(&bytes))
}

fn guard_current(reads: &ReadSession, root: &Path) -> bool {
    read_bounded(reads, &root.join(GUARD_PATH), MAX_SOURCE_BYTES)
        .is_ok_and(|bytes| bytes == GUARD_BYTES)
        && all_readers().all(|spec| reader_current(reads, root, spec))
}

fn reader_current(reads: &ReadSession, root: &Path, spec: &ReaderSpec) -> bool {
    read_bounded(reads, &root.join(spec.path), MAX_SOURCE_BYTES).is_ok_and(|bytes| {
        bytes == spec.bytes
            && (!contains_legacy_tokens(&bytes) || spec.legacy_tokens_are_negative_only)
    })
}

fn exact_manifest_state(reads: &ReadSession, root: &Path) -> bool {
    if !exact_agent_directory(reads, root) {
        return false;
    }
    let Ok(bytes) = read_bounded(reads, &root.join(MANIFEST_PATH), MAX_ARTIFACT_BYTES) else {
        return false;
    };
    if !crate::inventory::plugin_manifest_json::unique_keys(&bytes) {
        return false;
    }
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    let Some(rows) = value.get("agents").and_then(Value::as_array) else {
        return false;
    };
    if rows.len() != MANIFESTS.len()
        || !rows.iter().zip(MANIFESTS).all(|(row, spec)| {
            row.as_object().is_some_and(|object| {
                object.len() == 2
                    && row.get("name").and_then(Value::as_str) == Some(spec.name)
                    && row.get("path").and_then(Value::as_str) == Some(spec.path)
            })
        })
    {
        return false;
    }
    let paths = crate::package::inventory::inventory_paths(&value);
    if paths.iter().collect::<BTreeSet<_>>().len() != paths.len()
        || paths.iter().any(|path| legacy_agent_inventory_path(path))
        || paths
            .iter()
            .filter(|path| path.starts_with(".codex/agents/"))
            .map(String::as_str)
            .ne(MANIFESTS.iter().map(|spec| spec.path))
    {
        return false;
    }
    MANIFESTS.iter().all(|spec| {
        read_bounded(reads, &root.join(spec.path), MAX_ARTIFACT_BYTES).is_ok_and(|agent_bytes| {
            agent_bytes == spec.bytes
                && crate::agent_manifest::inspect(reads, &root.join(spec.path), spec.name)
                    .is_ok_and(|name| name == spec.name)
        })
    })
}

fn exact_agent_directory(reads: &ReadSession, root: &Path) -> bool {
    let Ok(entries) = collection_entries(reads, &root.join(".codex/agents")) else {
        return false;
    };
    if entries.len() != MANIFESTS.len()
        || entries
            .iter()
            .any(|entry| entry.kind != (CollectionEntryKind::Regular { single_link: true }))
    {
        return false;
    }
    let Ok(actual) = entries
        .iter()
        .map(|entry| relative(root, &entry.path))
        .collect::<Result<BTreeSet<_>, _>>()
    else {
        return false;
    };
    let expected = MANIFESTS
        .iter()
        .map(|spec| spec.path.to_owned())
        .collect::<BTreeSet<_>>();
    actual == expected
}

fn legacy_agent_inventory_path(value: &str) -> bool {
    let components = Path::new(value)
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    components.contains(&"custom-agents")
        || (components.contains(&"agents") && !value.starts_with(".codex/agents/"))
}

pub(crate) fn reader_proof_current(reads: &ReadSession, root: &Path) -> bool {
    phase_a_context_intact(reads, root)
        && guard_current(reads, root)
        && exact_manifest_state(reads, root)
        && no_unbound_reader(reads, root)
        && reads.revalidate().is_ok()
}

#[cfg(test)]
mod tests;
