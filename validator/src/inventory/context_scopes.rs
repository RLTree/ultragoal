mod evidence;
mod exact_set;
mod manifest;

use super::fs::{physical_regular_entry, read_bounded};
use super::types::{
    ActiveStatus, AuthorityState, InventoryEntry, InventoryError, InventoryFinding,
};
use crate::context::ReadSession;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(super) const REGISTRY_RELATIVE: &str = "migration/non-authoritative-contexts.json";
pub(super) const CANDIDATE_ROOT: &str = "docs/ultragoal-contract-2026-07-successor-candidate-v1";
const SCHEMA_VERSION: &str = "NonAuthoritativeContextRegistry-v1";
const SCHEMA_VERSION_V2: &str = "NonAuthoritativeContextRegistry-v2";
const CANDIDATE_CONTEXT_ID: &str = "successor-candidate-v1";
pub(super) const CANDIDATE_CONTRACT_ID: &str = "harness-ultragoal-successor-contract-candidate-v1";
pub(super) const CANDIDATE_STATUS: &str = "candidate_for_independent_review";
pub(super) const ZIP_MANIFEST_SHA256: &str =
    "2fb9b8e68c105ff92d6436801bf205d4cc528dd330292c89b0005909aab5231c";
pub(super) const CONTENT_SET_SHA256: &str =
    "f450645e73f0c324b2c9d8ca041142fff3ed4535c815ac80f87ed39c8a72d795";
pub(super) const CONTRACT_MANIFEST_SHA256: &str =
    "7489750e9a42ed6c50b64ec07c30242302b501956fdf4726ce56bbf6e680d91d";
const MAX_REGISTRY_BYTES: u64 = 256 * 1024;
const MAX_CONTEXT_ROWS: usize = 32;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema_version: String,
    contract_id: String,
    contexts: Vec<ContextRow>,
    #[serde(default)]
    exact_contexts: Vec<exact_set::ContextRow>,
    #[serde(default)]
    evidence_contexts: Vec<evidence::ContextRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextRow {
    context_id: String,
    root: String,
    zip_include_manifest_sha256: String,
    content_set_digest: String,
    contract_manifest_sha256: String,
    candidate_contract_id: String,
    authority: ContextAuthority,
    active_contract_replaced: bool,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContextAuthority {
    binding: bool,
}

pub(super) struct ContextScopes {
    pub(super) entries: Vec<InventoryEntry>,
    pub(super) findings: Vec<InventoryFinding>,
    verified_paths: BTreeSet<String>,
}

impl ContextScopes {
    fn empty() -> Self {
        Self {
            entries: Vec::new(),
            findings: Vec::new(),
            verified_paths: BTreeSet::new(),
        }
    }

    fn rejected(code: &str, path: &str, message: &str) -> Self {
        Self {
            entries: Vec::new(),
            findings: vec![InventoryFinding::error(
                code,
                None,
                Some(path),
                message.to_owned(),
            )],
            verified_paths: BTreeSet::new(),
        }
    }

    pub(super) fn remove_verified_legacy(&self, entries: &mut Vec<InventoryEntry>) {
        entries.retain(|entry| !self.verified_paths.contains(&entry.relative_path));
    }
}

fn exists(path: &Path) -> Result<bool, InventoryError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(InventoryError::Io {
            path: path.to_path_buf(),
            message: error.to_string(),
        }),
    }
}

fn supported_registry(registry: &Registry, contract_id: &str) -> bool {
    if registry.contract_id != contract_id
        || registry.contexts.is_empty()
        || registry.contexts.len() > MAX_CONTEXT_ROWS
        || registry.contexts.len() != 1
    {
        return false;
    }
    let Some(row) = registry.contexts.first() else {
        return false;
    };
    let candidate_valid = row.context_id == CANDIDATE_CONTEXT_ID
        && row.root == CANDIDATE_ROOT
        && row.zip_include_manifest_sha256 == ZIP_MANIFEST_SHA256
        && row.content_set_digest == CONTENT_SET_SHA256
        && row.contract_manifest_sha256 == CONTRACT_MANIFEST_SHA256
        && row.candidate_contract_id == CANDIDATE_CONTRACT_ID
        && !row.authority.binding
        && !row.active_contract_replaced
        && row.status == CANDIDATE_STATUS;
    candidate_valid
        && match registry.schema_version.as_str() {
            SCHEMA_VERSION => {
                registry.exact_contexts.is_empty() && registry.evidence_contexts.is_empty()
            }
            SCHEMA_VERSION_V2 => {
                exact_set::registry_valid(&registry.exact_contexts)
                    && evidence::registry_valid(&registry.evidence_contexts)
            }
            _ => false,
        }
}

fn context_entry(
    reads: &ReadSession,
    root: &Path,
    relative: &str,
    context_id: &str,
    kind: &str,
    proof_refs: Vec<String>,
) -> Result<InventoryEntry, InventoryError> {
    physical_regular_entry(
        reads,
        root,
        &root.join(relative),
        format!("CONTEXT:{context_id}:{relative}"),
        kind,
        "OWN-ULTRA-ROOT",
        AuthorityState::Context,
        ActiveStatus::ContextOnly,
        None,
        proof_refs,
        vec![format!("context-scope:{context_id}")],
    )
}

pub(super) fn load(
    reads: &ReadSession,
    root: &Path,
    current_contract_id: &str,
) -> Result<ContextScopes, InventoryError> {
    let registry_path = root.join(REGISTRY_RELATIVE);
    let candidate_path = root.join(CANDIDATE_ROOT);
    let has_registry = exists(&registry_path)?;
    let has_candidate = exists(&candidate_path)?;
    if !has_registry && !has_candidate {
        reads
            .observe_presence_parent(&registry_path)
            .map_err(|error| InventoryError::Io {
                path: registry_path,
                message: error.to_string(),
            })?;
        return Ok(ContextScopes::empty());
    }
    if !has_registry {
        return Err(InventoryError::InvalidRegistry(format!(
            "{REGISTRY_RELATIVE} is required while {CANDIDATE_ROOT} exists"
        )));
    }
    let bytes = match read_bounded(reads, &registry_path, MAX_REGISTRY_BYTES) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Ok(ContextScopes::rejected(
                "invalid_non_authoritative_context_registry",
                REGISTRY_RELATIVE,
                "the context registry is not a bounded regular file",
            ));
        }
    };
    let parsed = super::plugin_manifest_json::unique_keys(&bytes)
        .then(|| serde_json::from_slice::<Registry>(&bytes).ok())
        .flatten();
    let Some(registry) = parsed
        .as_ref()
        .filter(|registry| supported_registry(registry, current_contract_id))
    else {
        return Ok(ContextScopes::rejected(
            "invalid_non_authoritative_context_registry",
            REGISTRY_RELATIVE,
            "the context registry does not exactly bind the supported candidate scope",
        ));
    };
    let candidate_files = match manifest::verify(reads, root) {
        Ok(files) => files
            .into_iter()
            .map(|file| format!("{CANDIDATE_ROOT}/{file}"))
            .collect(),
        Err(_) => {
            return Ok(ContextScopes::rejected(
                "non_authoritative_context_verification_failed",
                CANDIDATE_ROOT,
                "the candidate context bundle failed exact manifest verification",
            ));
        }
    };
    let mut scoped = vec![(
        CANDIDATE_CONTEXT_ID,
        "non-authoritative-contract-context",
        candidate_files,
        vec![
            REGISTRY_RELATIVE.to_owned(),
            format!("{CANDIDATE_ROOT}/ZIP_INCLUDE_MANIFEST.json"),
        ],
    )];
    if registry.schema_version == SCHEMA_VERSION_V2 {
        let predecessor = match exact_set::verify(reads, root, &registry.exact_contexts[0]) {
            Ok(files) => files,
            Err(_) => {
                return Ok(ContextScopes::rejected(
                    "non_authoritative_context_verification_failed",
                    REGISTRY_RELATIVE,
                    "the predecessor contract context failed exact-set verification",
                ));
            }
        };
        scoped.push((
            exact_set::context_id(),
            "non-authoritative-contract-context",
            predecessor,
            vec![REGISTRY_RELATIVE.to_owned()],
        ));
        let worker_results = match evidence::verify(reads, root, &registry.evidence_contexts[0]) {
            Ok(files) => files,
            Err(_) => {
                return Ok(ContextScopes::rejected(
                    "non_authoritative_context_verification_failed",
                    REGISTRY_RELATIVE,
                    "the worker evidence context failed path and record verification",
                ));
            }
        };
        scoped.push((
            evidence::context_id(),
            "run-scoped-worker-evidence-context",
            worker_results,
            vec![
                REGISTRY_RELATIVE.to_owned(),
                evidence::schema_ref().to_owned(),
            ],
        ));
    }
    let mut result = ContextScopes::empty();
    result.entries.push(physical_regular_entry(
        reads,
        root,
        &registry_path,
        "CONTEXT-SCOPE-REGISTRY".to_owned(),
        "context-scope-registry",
        "OWN-ULTRA-ROOT",
        AuthorityState::Canonical,
        ActiveStatus::Active,
        None,
        Vec::new(),
        vec![format!("context-scope:{CANDIDATE_CONTEXT_ID}")],
    )?);
    for (context_id, kind, files, proof_refs) in scoped {
        for relative in files {
            result.entries.push(context_entry(
                reads,
                root,
                &relative,
                context_id,
                kind,
                proof_refs.clone(),
            )?);
            result.verified_paths.insert(relative);
        }
    }
    Ok(result)
}
