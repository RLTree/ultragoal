use super::*;

fn rust_tree(root: &Path) -> String {
    fn collect(directory: &Path, paths: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                collect(&path, paths);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                paths.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    collect(root, &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
pub(crate) fn production_mutation_grant_has_one_private_mint_in_the_sealed_authority() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/repository_fit");
    let authority = rust_tree(&source_root.join("product_adapter/authority"));
    let adapter = fs::read_to_string(source_root.join("product_adapter/mod.rs")).unwrap();
    let local = fs::read_to_string(source_root.join("local/mod.rs")).unwrap();
    let effects = rust_tree(&source_root.join("local/effects"));

    assert_eq!(authority.matches("struct LocalMutationGrant").count(), 1);
    assert_eq!(authority.matches("const fn issue() -> Self").count(), 1);
    assert_eq!(authority.matches("LocalMutationGrant::issue()").count(), 1);
    assert_eq!(authority.matches("pub(super) const fn issue").count(), 1);
    assert!(!authority.contains("pub(crate) const fn issue"));
    assert!(!authority.contains("pub(in crate::repository_fit) const fn issue"));
    assert!(!authority.contains("pub(crate) _private"));
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
pub(crate) fn historical_worker_result_is_self_consistent_and_keeps_its_unwired_claim_ceiling() {
    const RESULT_PATH: &str =
        "docs/ultragoal-successor-live/worker-results/REPOSITORY-FIT-PRODUCTION-AUTHORITY-085.json";
    const ARTIFACT_PATHS: [&str; 10] = [
        "validator/src/repository_fit/product_adapter.rs",
        "validator/src/repository_fit/product_adapter/root_permit.rs",
        "validator/src/repository_fit/product_adapter/authority.rs",
        "validator/src/repository_fit/product_adapter/ledger.rs",
        "validator/src/repository_fit/product_adapter/tests.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority.rs",
        "validator/src/repository_fit/mod.rs",
        "validator/src/repository_fit/local/mod.rs",
        "validator/src/repository_fit/local/unix.rs",
        "validator/src/repository_fit/local/effects.rs",
    ];
    const R5_TOUCHED_PATHS: [&str; 4] = [
        "validator/src/repository_fit/product_adapter/authority.rs",
        "validator/src/repository_fit/product_adapter/ledger.rs",
        "validator/src/repository_fit/product_adapter/root_permit.rs",
        "validator/src/repository_fit/product_adapter/tests/production_authority.rs",
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
    assert_eq!(
        result
            .final_state
            .get("status")
            .and_then(serde_json::Value::as_str),
        Some("candidate_for_root_acceptance")
    );
    assert_eq!(
        result
            .final_state
            .get("current_candidate_proof")
            .and_then(serde_json::Value::as_str),
        Some("established")
    );
    assert_eq!(
        result
            .final_state
            .get("public_apply_dispatch")
            .and_then(serde_json::Value::as_str),
        Some("unwired_root_only")
    );
    assert_eq!(
        result
            .candidate_identity
            .get("validation_source_head_commit")
            .and_then(serde_json::Value::as_str),
        Some("7b832340c4bddba12e4e92e7116aa526818d32d0")
    );
    assert_eq!(
        result
            .candidate_identity
            .get("validation_source_head_tree")
            .and_then(serde_json::Value::as_str),
        Some("a1ef56d16aedcce89379de18459bd658e087fa37")
    );

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
        assert!(artifact.byte_length > 0, "{}", artifact.path);
        assert!(artifact.sha256.starts_with("sha256:"), "{}", artifact.path);
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
    assert_eq!(
        result
            .final_state
            .get("artifact_set_sha256")
            .and_then(serde_json::Value::as_str),
        Some(artifact_set_sha256.as_str())
    );
    eprintln!(
        "worker_result_id={} artifact_set_sha256={artifact_set_sha256}",
        digest(&result_bytes)
    );
}
