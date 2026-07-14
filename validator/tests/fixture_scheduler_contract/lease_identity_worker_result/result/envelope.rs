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
        "validator/tests/fixture_scheduler_contract/lease_identity_worker_result/mod.rs",
        "validator/tests/fixture_scheduler_contract/scheduling.rs",
    ]
}

fn fixture_paths() -> Vec<&'static str> {
    vec![
        "validator/tests/fixture_scheduler_contract/adversarial.rs",
        "validator/tests/fixture_scheduler_contract/lease_identity_worker_result/mod.rs",
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
