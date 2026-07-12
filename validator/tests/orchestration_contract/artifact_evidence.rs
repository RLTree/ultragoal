use crate::orchestration::*;
use crate::support::*;
use std::collections::{BTreeMap, BTreeSet};

const EVIDENCE_DIR: &str = "docs/ultragoal-successor-live/frozen";
const EVIDENCE_FILE: &str = "docs/ultragoal-successor-live/frozen/input.json";

fn observed(path: &str) -> ArtifactRecord {
    ArtifactRecord {
        path: path.to_owned(),
        sha256: digest('d'),
        byte_length: 17,
    }
}

fn observed_subject(
    read_roots: &[&str],
    artifact: &str,
) -> (LeaseSpec, WorkPackage, WorkerResultV1) {
    let mut lease = lease();
    lease.read_paths = read_roots.iter().map(|value| path(value)).collect();
    let mut package = package("node-a", &[], "node_a");
    package.read_paths = lease.read_paths.clone();
    let mut result = result_for(&lease, &package);
    result.artifacts.push(observed(artifact));
    (lease, package, result)
}

#[test]
fn read_only_artifact_validates_under_exact_and_broad_read_paths() {
    for read_root in [EVIDENCE_FILE, EVIDENCE_DIR] {
        let (lease, package, result) = observed_subject(&[read_root], EVIDENCE_FILE);
        lease.validate(&policy()).unwrap();
        assert!(
            !result
                .touched_paths
                .iter()
                .any(|path| path == EVIDENCE_FILE)
        );
        result.validate_for(&lease, &package).unwrap();
        AcceptanceProposal::result_commitment_id_for(&lease.binding, &package, &lease, &result)
            .unwrap();
    }
}

#[test]
fn artifact_authority_is_lease_local_case_exact_and_segment_bounded() {
    for unauthorized in [
        "validator/src/orchestration/other.rs",
        "DOCS/ultragoal-successor-live/frozen/input.json",
        "docs/ultragoal-successor-live/frozen-copy/input.json",
        "docs/ultragoal-successor-live",
    ] {
        let (lease, mut package, result) = observed_subject(&[EVIDENCE_DIR], unauthorized);
        package
            .read_paths
            .insert(path("validator/src/orchestration"));
        assert_eq!(
            result.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::InvalidWorkerResult,
            "artifact {unauthorized} must use the exact current lease"
        );
    }
}

#[test]
fn malformed_and_host_protected_artifact_paths_fail_closed() {
    for malformed in ["../escape", "/absolute", "a/./b"] {
        let (lease, package, result) = observed_subject(&[EVIDENCE_DIR], malformed);
        assert_eq!(
            result.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::InvalidPath
        );
    }

    for protected in [".GIT", ".CODEX", ".AGENTS", ".CODEX-WORKTREE"] {
        let artifact = format!("{protected}/private/evidence.json");
        let (lease, package, result) = observed_subject(&[protected], &artifact);
        let mut expanded = policy();
        expanded.allowed_read_paths.insert(path(protected));
        assert_eq!(
            lease.validate(&expanded).unwrap_err(),
            OrchestrationError::RootOnlyScope
        );
        assert_eq!(
            result.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::RootOnlyScope
        );
    }
}

#[test]
fn read_authority_does_not_replace_touched_owned_or_fixture_requirements() {
    let mut written_lease = lease();
    let written = path("validator/src/orchestration/node_a.rs");
    written_lease.read_paths.insert(written.clone());
    let mut written_package = package("node-a", &[], "node_a");
    written_package.read_paths = written_lease.read_paths.clone();
    let result = result_for(&written_lease, &written_package);
    assert_eq!(
        result
            .validate_for(&written_lease, &written_package)
            .unwrap_err(),
        OrchestrationError::InvalidLease
    );
    let mut relabeled = result.clone();
    relabeled
        .touched_paths
        .retain(|value| value != written.as_str());
    assert_eq!(
        relabeled
            .validate_for(&written_lease, &written_package)
            .unwrap_err(),
        OrchestrationError::InvalidLease
    );

    let mut lease = lease();
    let mut package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    for raw in result
        .generated_outputs
        .iter()
        .chain(result.fixtures.iter())
    {
        lease.read_paths.insert(path(raw));
        package.read_paths = lease.read_paths.clone();
        let mut missing_touch = result.clone();
        missing_touch.touched_paths.retain(|value| value != raw);
        assert_eq!(
            missing_touch.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::InvalidWorkerResult
        );
    }
}

#[test]
fn duplicate_artifacts_stay_rejected_and_digest_mutation_changes_commitments() {
    let (lease, package, mut result) = observed_subject(&[EVIDENCE_DIR], EVIDENCE_FILE);
    let original_result_id = result.result_id().unwrap();
    let original_commitment =
        AcceptanceProposal::result_commitment_id_for(&lease.binding, &package, &lease, &result)
            .unwrap();
    result.artifacts.last_mut().unwrap().sha256 = digest('e');
    result.validate_for(&lease, &package).unwrap();
    assert_ne!(result.result_id().unwrap(), original_result_id);
    assert_ne!(
        AcceptanceProposal::result_commitment_id_for(&lease.binding, &package, &lease, &result)
            .unwrap(),
        original_commitment
    );

    result
        .artifacts
        .push(result.artifacts.last().unwrap().clone());
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
}

#[test]
fn rejected_observed_artifact_appends_no_submission_event() {
    let (mut engine, _) = engine();
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    result
        .artifacts
        .push(observed("validator/src/orchestration-other/evidence.rs"));
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    let before = engine.event_log();
    assert_eq!(
        engine
            .submit_structural(3, "lease-001", &result)
            .unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
    assert_eq!(engine.event_log(), before);
}
