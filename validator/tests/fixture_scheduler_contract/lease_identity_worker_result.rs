use crate::orchestration::*;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const RESULT: &str =
    "docs/ultragoal-successor-live/worker-results/FIXTURE-RECOVERY-STATE-CORRECTION-019.json";
const ENVELOPE: &str =
    "docs/ultragoal-successor-live/work-packages/FIXTURE-RECOVERY-STATE-CORRECTION-019.json";
const LEASE_ID: &str = "FIXTURE-RECOVERY-STATE-CORRECTION-019";
const WORKER: &str = "/root/fixture_cleanup_recovery_state_engineer";
const CONTEXT_ID: &str = "sha256:e16ec3d65c5cad916484db7a38218efdeb8e68410c1ddd40075bc5d44094dd16";
const ROOT_CANDIDATE_ID: &str =
    "sha256:44d6e42a0ef56a3d3dcd0c5dce97f450ca80f72ce959f35494b4fe434857bc55";
const ENVELOPE_NO_CLAIM_STATEMENT: &str = "This root-issued envelope grants no root authority and raises no fixture product, evaluation, readiness, release, or completion claim.";
const WORKER_NO_CLAIM_STATEMENT: &str =
    "This worker does not claim readiness, release, or completion.";
const ROOT_REWORK_DECISION: &str =
    "docs/ultragoal-successor-live/root-decisions/FIXTURE-RECOVERY-STATE-REWORK-019.json";
const ROOT_REWORK_DECISION_SHA256: &str =
    "sha256:e65de110e268abc4a70b693c95f9bfa913f292fef35f2d63bb2b2dae94a61604";

#[derive(Deserialize)]
struct PersistedEnvelope {
    work_package: WorkPackage,
    lease: LeaseSpec,
    no_claim_statement: String,
}

fn artifact_paths() -> Vec<&'static str> {
    vec![
        "validator/src/fixture_scheduler/lease.rs",
        "validator/src/fixture_scheduler/scheduler.rs",
    ]
}

fn fixture_paths() -> Vec<&'static str> {
    vec![
        "validator/tests/fixture_scheduler_contract/adversarial.rs",
        "validator/tests/fixture_scheduler_contract/lease_identity_worker_result.rs",
        "validator/tests/fixture_scheduler_contract/scheduling.rs",
    ]
}

fn all_owned_paths() -> Vec<&'static str> {
    artifact_paths()
        .into_iter()
        .chain(fixture_paths())
        .collect()
}

fn aggregate_id(root: &Path, paths: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for relative in paths {
        let bytes = fs::read(root.join(relative)).unwrap();
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn inputs() -> (PathBuf, Vec<u8>, PersistedEnvelope) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let result_bytes = fs::read(root.join(RESULT)).unwrap();
    let envelope_bytes = fs::read(root.join(ENVELOPE)).unwrap();
    let envelope = serde_json::from_slice(&envelope_bytes).unwrap();
    (root, result_bytes, envelope)
}

fn fixture_rows_verify(root: &Path, result: &WorkerResultV1) -> bool {
    let Some(rows) = result
        .final_state
        .get("fixture_artifact_rows")
        .and_then(Value::as_array)
    else {
        return false;
    };
    if rows.len() != fixture_paths().len() {
        return false;
    }
    fixture_paths()
        .into_iter()
        .zip(rows)
        .all(|(expected, row)| {
            let Some(path) = row.get("path").and_then(Value::as_str) else {
                return false;
            };
            let Some(sha256) = row.get("sha256").and_then(Value::as_str) else {
                return false;
            };
            let Some(byte_length) = row.get("byte_length").and_then(Value::as_u64) else {
                return false;
            };
            let Ok(bytes) = fs::read(root.join(expected)) else {
                return false;
            };
            path == expected
                && sha256 == format!("sha256:{:x}", Sha256::digest(&bytes))
                && byte_length == bytes.len() as u64
        })
}

#[test]
fn result_validates_for_persisted_lease_and_work_package_and_verifies_every_artifact() {
    let (root, bytes, envelope) = inputs();
    let parsed = WorkerResultV1::parse_json(&bytes).unwrap();
    let expected_paths: Vec<String> = artifact_paths().into_iter().map(str::to_owned).collect();
    let source_candidate = aggregate_id(&root, &artifact_paths());
    let owned_candidate = aggregate_id(&root, &all_owned_paths());

    envelope.work_package.validate().unwrap();
    envelope.lease.owned_scope.validate().unwrap();
    assert_eq!(envelope.lease.lease_id, LEASE_ID);
    assert_eq!(envelope.lease.owner, Actor::parse(WORKER).unwrap());
    assert_eq!(envelope.lease.binding.context_id, CONTEXT_ID);
    assert_eq!(envelope.lease.binding.candidate_id, ROOT_CANDIDATE_ID);
    assert_eq!(envelope.no_claim_statement, ENVELOPE_NO_CLAIM_STATEMENT);
    assert_eq!(parsed.touched_paths, expected_paths);
    assert!(parsed.fixtures.is_empty());
    assert_eq!(parsed.artifacts.len(), artifact_paths().len());
    assert_eq!(parsed.context_id, CONTEXT_ID);
    assert_eq!(
        parsed.candidate_identity.get("candidate_id"),
        Some(&Value::String(ROOT_CANDIDATE_ID.to_owned()))
    );
    assert_eq!(
        parsed
            .candidate_identity
            .get("worker_owned_artifact_set_sha256"),
        Some(&Value::String(source_candidate))
    );
    assert_eq!(
        parsed.final_state.get("worker_owned_artifact_set_sha256"),
        Some(&Value::String(owned_candidate))
    );
    assert_eq!(
        parsed
            .final_state
            .get("root_acceptance")
            .and_then(Value::as_str),
        Some("withheld")
    );
    assert_eq!(
        parsed.final_state.get("status").and_then(Value::as_str),
        Some("worker_blocked")
    );
    assert!(!parsed.unresolved_dependencies.is_empty());
    assert_eq!(parsed.requested_root_changes.len(), 1);
    assert_eq!(parsed.requested_root_changes[0].path, ROOT_REWORK_DECISION);
    assert_eq!(
        parsed.requested_root_changes[0].expected_sha256,
        ROOT_REWORK_DECISION_SHA256
    );
    assert_eq!(parsed.no_claim_statement, WORKER_NO_CLAIM_STATEMENT);

    parsed
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    let verified = ArtifactWorkspace::new(&root)
        .unwrap()
        .verify(&parsed, &envelope.lease, &envelope.work_package)
        .unwrap();
    assert_eq!(verified.result_id(), parsed.result_id().unwrap());
    assert_eq!(verified.artifact_count(), parsed.artifacts.len());
    assert!(fixture_rows_verify(&root, &parsed));
}

#[test]
fn stale_candidate_forged_artifact_scope_escape_and_inflated_claim_fail_closed() {
    let (root, bytes, envelope) = inputs();
    let parsed = WorkerResultV1::parse_json(&bytes).unwrap();

    let mut stale = parsed.clone();
    stale.candidate_identity.insert(
        "candidate_id".to_owned(),
        Value::String(format!("sha256:{}", "0".repeat(64))),
    );
    assert!(
        stale
            .validate_for(&envelope.lease, &envelope.work_package)
            .is_err()
    );

    let mut forged = parsed.clone();
    forged.artifacts[0].sha256 = format!("sha256:{}", "0".repeat(64));
    assert!(
        ArtifactWorkspace::new(&root)
            .unwrap()
            .verify(&forged, &envelope.lease, &envelope.work_package)
            .is_err()
    );

    let mut forged_fixture = parsed.clone();
    forged_fixture
        .final_state
        .get_mut("fixture_artifact_rows")
        .and_then(Value::as_array_mut)
        .unwrap()[0]["sha256"] = Value::String(format!("sha256:{}", "0".repeat(64)));
    assert!(!fixture_rows_verify(&root, &forged_fixture));

    let mut escaped = parsed.clone();
    escaped
        .touched_paths
        .push("validator/src/lib.rs".to_owned());
    assert!(
        escaped
            .validate_for(&envelope.lease, &envelope.work_package)
            .is_err()
    );

    let mut inflated = parsed;
    inflated.no_claim_statement = "fixture product cleanup is complete".to_owned();
    assert!(
        inflated
            .validate_for(&envelope.lease, &envelope.work_package)
            .is_err()
    );
}
