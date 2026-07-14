use super::*;

#[test]
pub(crate) fn production_mutation_grant_has_one_private_mint_in_the_sealed_authority() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repository_fit");
    let authority =
        fs::read_to_string(source_root.join("product_adapter/authority/mod.rs")).unwrap();
    let adapter = fs::read_to_string(source_root.join("product_adapter/mod.rs")).unwrap();
    let local = fs::read_to_string(source_root.join("local/mod.rs")).unwrap();
    let effects = fs::read_to_string(source_root.join("local/effects/mod.rs")).unwrap();

    assert_eq!(authority.matches("struct LocalMutationGrant").count(), 1);
    assert_eq!(authority.matches("const fn issue() -> Self").count(), 1);
    assert_eq!(authority.matches("LocalMutationGrant::issue()").count(), 1);
    assert!(!authority.contains("pub(crate) const fn issue"));
    assert!(!authority.contains("pub(crate) const fn issue"));
    assert!(!authority.contains("pub(in crate::repository_fit) const fn issue"));
    assert!(adapter.contains("pub(in crate::repository_fit) use authority::LocalMutationGrant;"));
    assert!(!local.contains("LocalMutationGrant"));
    assert!(!local.contains("issue_local_mutation_grant"));
    assert_eq!(
        effects
            .matches("use crate::repository_fit::product_adapter::LocalMutationGrant;")
            .count(),
        2
    );
    assert_eq!(effects.matches("_grant: LocalMutationGrant").count(), 2);
}

#[test]
pub(crate) fn worker_result_is_typed_and_freezes_the_exact_regular_single_link_artifact_set() {
    const RESULT_PATH: &str =
        "docs/ultragoal-successor-live/worker-results/REPOSITORY-FIT-PRODUCTION-AUTHORITY-085.json";
    const ARTIFACT_PATHS: [&str; 10] = [
        "validator/src/repository_fit/product_adapter/mod.rs",
        "validator/src/repository_fit/product_adapter/root_permit/mod.rs",
        "validator/src/repository_fit/product_adapter/authority/mod.rs",
        "validator/src/repository_fit/product_adapter/ledger/mod.rs",
        "validator/src/repository_fit/product_adapter/tests/mod.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority/mod.rs",
        "validator/src/repository_fit/mod.rs",
        "validator/src/repository_fit/local/mod.rs",
        "validator/src/repository_fit/local/unix/mod.rs",
        "validator/src/repository_fit/local/effects/mod.rs",
    ];
    const R5_TOUCHED_PATHS: [&str; 4] = [
        "validator/src/repository_fit/product_adapter/authority/mod.rs",
        "validator/src/repository_fit/product_adapter/ledger/mod.rs",
        "validator/src/repository_fit/product_adapter/root_permit/mod.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority/mod.rs",
    ];

    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let result_bytes = fs::read(workspace.join(RESULT_PATH)).unwrap();
    assert!(!result_bytes.is_empty() && result_bytes.len() <= 4 * 1024 * 1024);
    let result: ProofWorkerResultV1 = serde_json::from_slice(&result_bytes).unwrap();
    assert_eq!(
        result.worker,
        "/root/repository_fit_proof_recovery_engineer"
    );
    assert_eq!(
        result.lease_id,
        "REPOSITORY-FIT-ROOT-BINDING-RECOVERY-085-R5"
    );
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
    let status = result
        .final_state
        .get("status")
        .and_then(serde_json::Value::as_str);
    let blocker = result.final_state.get("blocker");
    let current_candidate_proof = result
        .final_state
        .get("current_candidate_proof")
        .and_then(serde_json::Value::as_str);
    match status {
        Some("worker_blocked") => {
            assert_eq!(
                blocker.and_then(serde_json::Value::as_str),
                Some("current_candidate_validation_enospc")
            );
            assert_eq!(current_candidate_proof, Some("not_established"));
        }
        Some("candidate_for_root_acceptance") => {
            assert!(
                blocker.is_none(),
                "accepted candidate cannot retain a blocker"
            );
            assert_eq!(current_candidate_proof, Some("established"));
        }
        other => panic!("unrecognized repository-fit WorkerResult status: {other:?}"),
    }

    let mut touched = result.touched_paths.clone();
    touched.sort();
    let mut expected_touched = R5_TOUCHED_PATHS.map(str::to_owned).to_vec();
    expected_touched.push(RESULT_PATH.to_owned());
    expected_touched.sort();
    assert_eq!(touched, expected_touched);
    assert_eq!(result.generated_outputs, [RESULT_PATH]);
    assert!(result.fixtures.is_empty());

    let artifact_paths = result
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(artifact_paths, ARTIFACT_PATHS);
    for artifact in &result.artifacts {
        let path = workspace.join(&artifact.path);
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert!(
            metadata.is_file(),
            "{} is not a regular file",
            artifact.path
        );
        assert_eq!(metadata.nlink(), 1, "{} is not single-link", artifact.path);
        let bytes = fs::read(path).unwrap();
        assert_eq!(
            bytes.len() as u64,
            artifact.byte_length,
            "{}",
            artifact.path
        );
        assert_eq!(digest(&bytes), artifact.sha256, "{}", artifact.path);
    }

    let mut canonical_artifacts = result.artifacts.iter().collect::<Vec<_>>();
    canonical_artifacts.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    let mut canonical_artifact_set = Vec::new();
    for artifact in canonical_artifacts {
        canonical_artifact_set.extend_from_slice(artifact.path.as_bytes());
        canonical_artifact_set.push(b'\t');
        canonical_artifact_set.extend_from_slice(
            artifact
                .sha256
                .strip_prefix("sha256:")
                .expect("artifact digests use the canonical sha256: prefix")
                .as_bytes(),
        );
        canonical_artifact_set.push(b'\n');
    }
    let artifact_set_sha256 = digest(&canonical_artifact_set);
    assert_eq!(
        result
            .candidate_identity
            .get("artifact_set_formula")
            .and_then(serde_json::Value::as_str),
        Some(
            "Sort artifact rows by UTF-8 path bytes, serialize each as path<TAB>lowercase SHA-256 without the sha256: prefix<LF>, concatenate, then SHA-256 the resulting bytes."
        )
    );
    assert_eq!(
        result
            .candidate_identity
            .get("artifact_set_sha256")
            .and_then(serde_json::Value::as_str),
        Some(artifact_set_sha256.as_str())
    );
    eprintln!(
        "worker_result_id={} artifact_set_sha256={artifact_set_sha256}",
        digest(&result_bytes)
    );
}
