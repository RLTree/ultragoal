use crate::orchestration::*;
use crate::orchestration_fixture::*;

fn observed(path: &str) -> ArtifactRecord {
    ArtifactRecord {
        path: path.to_owned(),
        sha256: digest('d'),
        byte_length: 17,
    }
}

fn read_result(paths: &[&str]) -> (LeaseSpec, WorkPackage, WorkerResultV1) {
    let mut lease = lease();
    lease.read_paths.insert(path("evidence"));
    let mut package = package("node-a", &[], "node_a");
    package.read_paths = lease.read_paths.clone();
    let mut result = result_for(&lease, &package);
    result
        .artifacts
        .extend(paths.iter().map(|value| observed(value)));
    (lease, package, result)
}

#[test]
fn exact_casefold_and_segment_alias_artifacts_fail_closed() {
    for paths in [
        vec!["evidence/a.json", "evidence/a.json"],
        vec!["evidence/A.json", "evidence/a.json"],
        vec!["evidence/a", "evidence/a/child.json"],
    ] {
        let (lease, package, result) = read_result(&paths);
        assert_eq!(
            result.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::DuplicateOutput,
            "aliases {paths:?} must not enter a commitment"
        );
    }
}

#[test]
fn segment_bounded_sibling_prefixes_remain_distinct() {
    let (lease, package, result) = read_result(&["evidence/a", "evidence/ab"]);
    result.validate_for(&lease, &package).unwrap();
}

#[test]
fn unicode_normalization_ambiguity_is_rejected_before_identity() {
    for ambiguous in ["evidence/caf\u{e9}.json", "evidence/cafe\u{301}.json"] {
        let (lease, package, result) = read_result(&[ambiguous]);
        assert_eq!(
            result.validate_for(&lease, &package).unwrap_err(),
            OrchestrationError::InvalidPath
        );
    }
}

#[test]
fn current_lease_cannot_consume_another_leases_owned_artifact() {
    let (lease, package, mut result) = read_result(&[]);
    result
        .artifacts
        .push(observed("validator/src/orchestration/other.rs"));
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn case_alias_touch_declarations_cannot_bypass_unique_identity() {
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let mut result = result_for(&lease, &package);
    result
        .touched_paths
        .push("VALIDATOR/SRC/ORCHESTRATION/NODE_A.RS".to_owned());
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::DuplicateOutput
    );
}

#[test]
fn absolute_traversal_ancestor_and_host_protected_paths_fail_closed() {
    for raw in [
        "/absolute",
        "../escape",
        "evidence/../escape",
        ".GIT/config",
        ".CODEX/private",
        ".AGENTS/private",
        ".CODEX-WORKTREE/ENV.SH",
    ] {
        let (lease, package, result) = read_result(&[raw]);
        let error = result.validate_for(&lease, &package).unwrap_err();
        assert!(matches!(
            error,
            OrchestrationError::InvalidPath | OrchestrationError::RootOnlyScope
        ));
    }
}
