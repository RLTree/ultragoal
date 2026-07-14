use crate::review::round::ReviewFailure;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

mod reader;
mod semantics;
mod validation;

const MAX_AGENT_MANIFEST_BYTES: u64 = 64 * 1024;

#[cfg(all(test, unix))]
pub(crate) use reader::{set_after_read_hook, set_before_final_revalidate_hook};

#[derive(Default)]
pub(crate) struct RegistrySnapshot {
    stable: bool,
    manifest_digests: BTreeMap<&'static str, String>,
}

impl RegistrySnapshot {
    pub(crate) fn manifest_digest(&self, role: &str, path: &str) -> Option<&str> {
        self.stable
            .then(|| self.manifest_digests.get(role))
            .flatten()
            .filter(|_| {
                crate::review::round::config::review_role_spec(role)
                    .is_some_and(|spec| spec.agent_manifest_path == path)
            })
            .map(String::as_str)
    }
}

pub(crate) fn exposure_errors(
    root: &Path,
    receipt: &Value,
    out: &mut Vec<ReviewFailure>,
) -> RegistrySnapshot {
    let Some(artifact) = receipt.get("live_registry_exposure") else {
        out.push(failure(
            "review_round_live_registry_exposure_missing",
            "receipt",
        ));
        return RegistrySnapshot::default();
    };
    let rel = artifact.get("path").and_then(Value::as_str).unwrap_or("");
    if crate::package::inventory::package_path_error(root, rel).is_some() {
        out.push(failure(
            "review_round_live_registry_artifact_invalid",
            "registry-exposure",
        ));
        return RegistrySnapshot::default();
    }
    let mut session = match reader::Session::new(root) {
        Ok(session) => session,
        Err(_) => {
            out.push(failure(
                "review_round_live_registry_artifact_invalid",
                "registry-exposure",
            ));
            return RegistrySnapshot::default();
        }
    };
    let Some(exposure) = read_exposure(&mut session, rel, artifact, out) else {
        return RegistrySnapshot::default();
    };
    let Some(schema) = read_schema(&mut session, out) else {
        return RegistrySnapshot::default();
    };
    let schema_errors = semantics::schema_errors(&schema.bytes, &schema.value, &exposure);
    if !schema_errors.is_empty() {
        out.push(failure(
            "review_round_live_registry_artifact_malformed",
            "registry-exposure-schema",
        ));
        return RegistrySnapshot::default();
    }
    let Some(mut snapshot) = read_manifests(&mut session, out) else {
        return RegistrySnapshot::default();
    };
    if session.revalidate().is_err() {
        out.push(failure(
            "review_round_live_registry_artifact_mismatch",
            "registry-read-session-identity",
        ));
        return RegistrySnapshot::default();
    }
    snapshot.stable = true;
    validation::exposure_shape_errors(&snapshot, &exposure, out);
    validation::exposure_identity_errors(receipt, &exposure, out);
    validation::spawn_session_errors(receipt, &exposure, out);
    if exposure.get("status").and_then(Value::as_str) != Some("fail")
        || exposure.get("claim_ceiling").and_then(Value::as_str) != Some("withheld_or_blocked")
    {
        out.push(failure(
            "review_round_live_registry_artifact_malformed",
            "registry-positive-proof-impossible",
        ));
    }
    out.push(failure(
        "review_round_live_registry_unavailable",
        "runtime-and-discovery",
    ));
    snapshot
}

fn read_exposure(
    session: &mut reader::Session,
    rel: &str,
    artifact: &Value,
    out: &mut Vec<ReviewFailure>,
) -> Option<Value> {
    let bounded = match session.read_json(rel) {
        Ok(bounded) => bounded,
        Err(reader::Error::Invalid) => {
            out.push(failure(
                "review_round_live_registry_artifact_invalid",
                "registry-exposure",
            ));
            return None;
        }
        Err(reader::Error::Changed) => {
            out.push(failure(
                "review_round_live_registry_artifact_mismatch",
                "registry-exposure-identity",
            ));
            return None;
        }
        Err(reader::Error::Malformed) => {
            out.push(failure(
                "review_round_live_registry_artifact_malformed",
                "registry-exposure-json",
            ));
            return None;
        }
    };
    let want = artifact.get("digest").and_then(Value::as_str).unwrap_or("");
    if crate::digest::bytes(&bounded.bytes) != want {
        out.push(failure(
            "review_round_live_registry_artifact_mismatch",
            "registry-exposure-digest",
        ));
        return None;
    }
    Some(bounded.value)
}

fn read_schema(
    session: &mut reader::Session,
    out: &mut Vec<ReviewFailure>,
) -> Option<reader::Artifact> {
    match session.read_json(semantics::SCHEMA_PATH) {
        Ok(schema) => Some(schema),
        Err(_) => {
            out.push(failure(
                "review_round_live_registry_schema_unavailable",
                "registry-schema",
            ));
            None
        }
    }
}

fn read_manifests(
    session: &mut reader::Session,
    out: &mut Vec<ReviewFailure>,
) -> Option<RegistrySnapshot> {
    let mut snapshot = RegistrySnapshot::default();
    for spec in crate::review::round::config::REVIEW_ROLES {
        let bytes = match session.read_bytes(spec.agent_manifest_path, MAX_AGENT_MANIFEST_BYTES) {
            Ok(bytes) => bytes,
            Err(reader::Error::Changed) => {
                out.push(failure(
                    "review_round_live_registry_artifact_mismatch",
                    "registry-read-session-identity",
                ));
                return None;
            }
            Err(_) => {
                out.push(failure(
                    "review_round_live_registry_agent_mismatch",
                    spec.role_name,
                ));
                return None;
            }
        };
        if !semantics::manifest_is_current(&bytes, spec.role_name) {
            out.push(failure(
                "review_round_live_registry_agent_mismatch",
                spec.role_name,
            ));
            return None;
        }
        snapshot
            .manifest_digests
            .insert(spec.role_name, crate::digest::bytes(&bytes));
    }
    Some(snapshot)
}

pub(crate) fn row_agent_role_error(row: &Value, role: &str, out: &mut Vec<ReviewFailure>) {
    validation::row_agent_role_error(row, role, out);
}

fn failure(code: &str, detail: impl Into<String>) -> ReviewFailure {
    ReviewFailure::new("validator-execution-provenance", code, detail)
}
