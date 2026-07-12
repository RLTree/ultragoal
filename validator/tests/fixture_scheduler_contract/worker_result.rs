use crate::orchestration::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

const RESULT: &str = "docs/ultragoal-successor-live/worker-results/FIXTURE-DETACHED-DESCENDANT-CONFINEMENT-CORRECTION-009.json";
const DECISION: &str = "docs/ultragoal-successor-live/root-decisions/FIXTURE-SCHEDULER-DETACHED-DESCENDANT-REWORK.json";
const LEASE_ID: &str = "FIXTURE-DETACHED-DESCENDANT-CONFINEMENT-CORRECTION-009";
const WORKER: &str = "/root/detached_descendant_confinement_builder";

fn path(value: &str) -> CanonicalPath {
    CanonicalPath::parse(value).unwrap()
}

fn source_paths() -> Vec<String> {
    [
        "validator/src/cli/capture/fixture/execute.rs",
        "validator/src/fixture_scheduler/confinement/backend.rs",
        "validator/src/fixture_scheduler/confinement/policy.rs",
        "validator/tests/fixture_scheduler_contract/confinement.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/detached_descendant.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/process_group.rs",
        "validator/tests/fixture_scheduler_contract/helpers/confinement_probe.rs",
        "validator/tests/fixture_scheduler_contract/worker_result.rs",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn subject(context: &str, candidate: &str) -> (LeaseSpec, WorkPackage) {
    let paths = source_paths();
    let owned = OwnedScope {
        paths: paths
            .iter()
            .map(|item| path(item))
            .chain(std::iter::once(path(RESULT)))
            .collect(),
        semantic_symbols: BTreeSet::from([
            "fixture_scheduler::detached_descendant_confinement".to_owned()
        ]),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: BTreeSet::from([
            EffectGrant::new(EffectClass::WorkspaceWrite, "fixture-detached-source").unwrap(),
            EffectGrant::new(EffectClass::FixtureWrite, "fixture-detached-tests").unwrap(),
            EffectGrant::new(EffectClass::Process, "fixture-detached-validation").unwrap(),
        ]),
    };
    let read_paths = BTreeSet::from([path(DECISION)]);
    let binding = Binding::new(context, candidate).unwrap();
    let lease = LeaseSpec {
        lease_id: LEASE_ID.to_owned(),
        run_id: "fixture-detached-descendant-confinement-009".to_owned(),
        node_id: "fixture-detached-descendant-confinement".to_owned(),
        principal: Principal::Worker,
        owner: Actor::parse(WORKER).unwrap(),
        binding,
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: read_paths.clone(),
        owned_scope: owned.clone(),
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 2,
        max_retries: 0,
    };
    let package = WorkPackage {
        node_id: lease.node_id.clone(),
        dependencies: BTreeSet::from(["fixture-scheduler-detached-descendant-rework".to_owned()]),
        required_tools: BTreeSet::from(["cargo".to_owned(), "rustfmt".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths,
        owned_scope: owned,
        prerequisites: BTreeSet::new(),
        outputs: BTreeSet::from(["fixture-detached-confinement-candidate".to_owned()]),
        acceptance: BTreeSet::from(["fixture-detached-confinement-009".to_owned()]),
        claim_effect: "private-darwin-fixture-confinement-only".to_owned(),
    };
    (lease, package)
}

fn inputs() -> (PathBuf, Vec<u8>, LeaseSpec, WorkPackage) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let decision_bytes = fs::read(root.join(DECISION)).unwrap();
    let decision: Value = serde_json::from_slice(&decision_bytes).unwrap();
    let context = format!("sha256:{:x}", Sha256::digest(&decision_bytes));
    let candidate = format!(
        "sha256:{}",
        decision
            .pointer("/reviewed_candidate/artifact_set_sha256")
            .and_then(Value::as_str)
            .unwrap()
    );
    let result_bytes = fs::read(root.join(RESULT)).unwrap();
    let (lease, package) = subject(&context, &candidate);
    (root, result_bytes, lease, package)
}

#[test]
fn worker_result_is_parseable_scope_valid_workspace_verified_and_result_identified() {
    let (root, bytes, lease, package) = inputs();
    let parsed = WorkerResultV1::parse_json(&bytes).unwrap();
    assert_eq!(parsed.touched_paths, source_paths());
    assert_eq!(parsed.artifacts.len(), source_paths().len());
    parsed.validate_for(&lease, &package).unwrap();
    let verified = ArtifactWorkspace::new(&root)
        .unwrap()
        .verify(&parsed, &lease, &package)
        .unwrap();
    assert_eq!(verified.result_id(), parsed.result_id().unwrap());
    assert_eq!(verified.artifact_count(), parsed.artifacts.len());
}

#[test]
fn worker_result_negative_mutations_fail_closed() {
    let (root, bytes, lease, package) = inputs();
    let parsed = WorkerResultV1::parse_json(&bytes).unwrap();

    let mut unknown: Value = serde_json::from_slice(&bytes).unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert("unexpected".to_owned(), Value::Bool(true));
    assert!(WorkerResultV1::parse_json(&serde_json::to_vec(&unknown).unwrap()).is_err());

    let mut stale = parsed.clone();
    stale.context_id = format!("sha256:{}", "0".repeat(64));
    assert!(stale.validate_for(&lease, &package).is_err());

    let mut forged = parsed.clone();
    forged.artifacts[0].sha256 = format!("sha256:{}", "0".repeat(64));
    assert!(
        ArtifactWorkspace::new(&root)
            .unwrap()
            .verify(&forged, &lease, &package)
            .is_err()
    );

    let original_id = parsed.result_id().unwrap();
    let mut changed = parsed;
    changed.limitations.push("negative-mutation".to_owned());
    assert_ne!(original_id, changed.result_id().unwrap());
}
