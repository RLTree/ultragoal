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
const CORRECTION_RESULT: &str =
    "docs/ultragoal-successor-live/worker-results/FIXTURE-RESULT-COMMITMENT-CORRECTION-026.json";
const CORRECTION_ENVELOPE: &str =
    "docs/ultragoal-successor-live/work-packages/FIXTURE-RESULT-COMMITMENT-CORRECTION-026.json";
const CORRECTION_LEASE_ID: &str = "FIXTURE-RESULT-COMMITMENT-CORRECTION-026";
const CORRECTION_WORKER: &str = "/root/fixture_result_commitment_engineer";
const CORRECTION_CONTEXT_ID: &str =
    "sha256:d8e64dce3fdd4ba8d58570d081a37dc298411f4ff921c676a74d63a07b9fc69a";
const CORRECTION_CANDIDATE_ID: &str =
    "sha256:fa5ebea77793fe008f6ad0d4549b9ed2ef7fcd317c890c28444f41abfcae094e";

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
        "validator/tests/fixture_scheduler_contract/adversarial.rs",
        "validator/tests/fixture_scheduler_contract/lease_identity_worker_result.rs",
        "validator/tests/fixture_scheduler_contract/scheduling.rs",
    ]
}

fn fixture_paths() -> Vec<&'static str> {
    vec![
        "validator/tests/fixture_scheduler_contract/adversarial.rs",
        "validator/tests/fixture_scheduler_contract/lease_identity_worker_result.rs",
        "validator/tests/fixture_scheduler_contract/scheduling.rs",
    ]
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

fn correction_inputs(root: &Path) -> (Vec<u8>, PersistedEnvelope) {
    let result_bytes = fs::read(root.join(CORRECTION_RESULT)).unwrap();
    let envelope_bytes = fs::read(root.join(CORRECTION_ENVELOPE)).unwrap();
    let envelope = serde_json::from_slice(&envelope_bytes).unwrap();
    (result_bytes, envelope)
}

fn artifact_paths_from(result: &WorkerResultV1) -> Vec<&str> {
    result
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect()
}

fn commitment_id(
    result: &WorkerResultV1,
    envelope: &PersistedEnvelope,
) -> Result<String, OrchestrationError> {
    AcceptanceProposal::result_commitment_id_for(
        &envelope.lease.binding,
        &envelope.work_package,
        &envelope.lease,
        result,
    )
}

#[test]
fn result_validates_for_persisted_lease_and_work_package_and_verifies_every_artifact() {
    let (root, bytes, envelope) = inputs();
    let parsed = WorkerResultV1::parse_json(&bytes).unwrap();
    let expected_paths: Vec<String> = artifact_paths().into_iter().map(str::to_owned).collect();
    let expected_fixtures: Vec<String> = fixture_paths().into_iter().map(str::to_owned).collect();
    let owned_candidate = aggregate_id(&root, &artifact_paths());

    envelope.work_package.validate().unwrap();
    envelope.lease.owned_scope.validate().unwrap();
    assert_eq!(envelope.lease.lease_id, LEASE_ID);
    assert_eq!(envelope.lease.owner, Actor::parse(WORKER).unwrap());
    assert_eq!(envelope.lease.binding.context_id, CONTEXT_ID);
    assert_eq!(envelope.lease.binding.candidate_id, ROOT_CANDIDATE_ID);
    assert_eq!(envelope.no_claim_statement, ENVELOPE_NO_CLAIM_STATEMENT);
    assert_eq!(parsed.touched_paths, expected_paths);
    assert_eq!(parsed.fixtures, expected_fixtures);
    assert_eq!(parsed.artifacts.len(), artifact_paths().len());
    assert_eq!(artifact_paths_from(&parsed), artifact_paths());
    assert_eq!(parsed.context_id, CONTEXT_ID);
    assert_eq!(
        parsed.candidate_identity.get("candidate_id"),
        Some(&Value::String(ROOT_CANDIDATE_ID.to_owned()))
    );
    assert_eq!(
        parsed
            .candidate_identity
            .get("worker_owned_artifact_set_sha256"),
        Some(&Value::String(owned_candidate.clone()))
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
    assert_eq!(verified.artifact_count(), 5);
    assert!(parsed.final_state.get("fixture_artifact_rows").is_none());

    let canonical_commitment = commitment_id(&parsed, &envelope).unwrap();
    for index in 0..parsed.artifacts.len() {
        let mut mutated = parsed.clone();
        mutated.artifacts[index].sha256 = format!("sha256:{}", "0".repeat(64));
        assert_ne!(
            commitment_id(&mutated, &envelope).unwrap(),
            canonical_commitment,
            "artifact row {index} must contribute to ResultCommitment"
        );
    }

    let (correction_bytes, correction_envelope) = correction_inputs(&root);
    let correction = WorkerResultV1::parse_json(&correction_bytes).unwrap();
    correction_envelope.work_package.validate().unwrap();
    correction_envelope.lease.owned_scope.validate().unwrap();
    assert_eq!(correction.lease_id, CORRECTION_LEASE_ID);
    assert_eq!(correction.worker, CORRECTION_WORKER);
    assert_eq!(correction.context_id, CORRECTION_CONTEXT_ID);
    assert_eq!(
        correction_envelope.lease.binding.candidate_id,
        CORRECTION_CANDIDATE_ID
    );
    assert_eq!(correction.artifacts.len(), 2);
    assert_eq!(correction.fixtures, vec![artifact_paths()[3].to_owned()]);
    correction
        .validate_for(
            &correction_envelope.lease,
            &correction_envelope.work_package,
        )
        .unwrap();
    let correction_verified = ArtifactWorkspace::new(&root)
        .unwrap()
        .verify(
            &correction,
            &correction_envelope.lease,
            &correction_envelope.work_package,
        )
        .unwrap();
    assert_eq!(correction_verified.artifact_count(), 2);
}

#[test]
fn stale_candidate_forged_artifact_scope_escape_and_inflated_claim_fail_closed() {
    let (root, bytes, envelope) = inputs();
    let parsed = WorkerResultV1::parse_json(&bytes).unwrap();
    let canonical_commitment = commitment_id(&parsed, &envelope).unwrap();

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
    assert_ne!(
        commitment_id(&forged, &envelope).unwrap(),
        canonical_commitment
    );

    let mut missing = parsed.clone();
    missing.artifacts.pop();
    assert_ne!(artifact_paths_from(&missing), artifact_paths());
    assert_ne!(
        commitment_id(&missing, &envelope).unwrap(),
        canonical_commitment
    );

    let mut unknown = parsed.clone();
    let unknown_path = "validator/src/orchestration/artifact.rs";
    let unknown_bytes = fs::read(root.join(unknown_path)).unwrap();
    unknown.artifacts.push(ArtifactRecord {
        path: unknown_path.to_owned(),
        sha256: format!("sha256:{:x}", Sha256::digest(&unknown_bytes)),
        byte_length: unknown_bytes.len() as u64,
    });
    assert_ne!(artifact_paths_from(&unknown), artifact_paths());
    assert!(commitment_id(&unknown, &envelope).is_err());

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
