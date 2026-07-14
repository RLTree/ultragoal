use super::*;

#[cfg(unix)]
#[test]
pub(crate) fn omitted_or_injected_transitive_inputs_and_definition_only_targets_cannot_pass() {
    let root = TestRoot::new("input-set-inexact", VALID_CATALOG);
    let catalog = load_full(&root, CANDIDATE_ID);
    let mut rows = selected(&root, false);
    rows[1] = SelectedRoutineNode::new(
        "verify",
        vec!["syntax".to_owned()],
        sha(b"verify input identity"),
        vec![input(&root, "src/input.txt")],
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        rows,
        vec![runner(
            "ultragoal",
            TRUE_TOOL_ID,
            Path::new("/usr/bin/true"),
        )],
    )
    .unwrap();
    assert_eq!(
        catalog.bind_selected(request).unwrap_err().code(),
        "catalog-selection-transitive-input-set-inexact"
    );

    assert_eq!(
        CatalogSelectionRequest::new(
            catalog.catalog_id(),
            GRAPH_ID,
            CANDIDATE_ID,
            PLAN_ID,
            Vec::new(),
            Vec::new(),
        )
        .unwrap_err()
        .code(),
        "catalog-selection-cardinality-invalid"
    );

    let extra_runner = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![
            runner("ultragoal", TRUE_TOOL_ID, Path::new("/usr/bin/true")),
            runner("other", OTHER_CANDIDATE_ID, Path::new("/usr/bin/false")),
        ],
    )
    .unwrap_err();
    assert_eq!(
        extra_runner.code(),
        "catalog-runner-observation-set-inexact"
    );
}

#[cfg(unix)]
#[test]
pub(crate) fn parse_query_and_refusal_paths_are_recursively_zero_write() {
    let root = TestRoot::new("zero-write", VALID_CATALOG);
    commit_fixture(root.path());
    let before = tree(root.path());
    let before_status = status(root.path());
    let catalog = load_full(&root, CANDIDATE_ID);
    assert_eq!(catalog.graph_id(), GRAPH_ID);
    assert_eq!(catalog.definition_count(), 2);
    assert_eq!(tree(root.path()), before);
    assert_eq!(status(root.path()), before_status);

    let metadata = fs::metadata("/usr/bin/true").unwrap();
    let forged = RunnerObservation::new(
        "ultragoal",
        TRUE_TOOL_ID,
        "/usr/bin/true",
        sha(b"forged"),
        metadata.len(),
        metadata.mode(),
    )
    .unwrap();
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        GRAPH_ID,
        CANDIDATE_ID,
        PLAN_ID,
        selected(&root, false),
        vec![forged],
    )
    .unwrap();
    assert!(catalog.bind_selected(request).is_err());
    assert_eq!(tree(root.path()), before);
    assert_eq!(status(root.path()), before_status);
}

#[test]
pub(crate) fn corrective_worker_result_is_typed_bound_artifact_verified_and_substitution_safe() {
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert_eq!(
        file_sha(&repository_root.join(R3_WORK_PACKAGE_PATH)),
        R3_WORK_PACKAGE_SHA256,
        "receipt is bound to the exact root-issued R3 work package"
    );
    let result_bytes = fs::read(repository_root.join(R3_RESULT_PATH)).unwrap();
    let result =
        WorkerResultV1::parse_json(&result_bytes).expect("authoritative WorkerResultV1 parser");
    let (package, lease, policy) = corrective_lease();
    package.validate().expect("typed corrective work package");
    policy.validate().expect("typed corrective scope policy");
    lease
        .validate(&policy)
        .expect("typed corrective lease and scope binding");
    result
        .validate_for(&lease, &package)
        .expect("typed corrective WorkerResult binding");

    let before = result
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                fs::read(repository_root.join(&artifact.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let workspace = ArtifactWorkspace::new(repository_root).unwrap();
    let verified = workspace
        .verify(&result, &lease, &package)
        .expect("ArtifactWorkspace exact artifact verification");
    let result_id = result.result_id().expect("canonical result identity");
    assert_eq!(verified.result_id(), result_id);
    assert_eq!(verified.artifact_count(), 3);
    assert!(r3_artifact_set_is_exact(&result));
    assert_eq!(result.context_id, R3_CONTEXT_ID);
    assert_eq!(result.candidate_identity["context_id"], R3_CONTEXT_ID);
    assert_eq!(result.candidate_identity["candidate_id"], R3_CANDIDATE_ID);
    assert_eq!(
        result.candidate_identity["issued_root_commit"],
        "1750d586f6561d9d6ba64887cdafa6a36f23e41e"
    );
    assert_eq!(
        result.candidate_identity["issued_root_tree"],
        "bf2d3e53c4b9410df41fb94ddf9b56ea4cf1e58a"
    );
    assert_eq!(
        result.candidate_identity["work_package_sha256"],
        R3_WORK_PACKAGE_SHA256
    );
    let after = result
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                fs::read(repository_root.join(&artifact.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(after, before, "receipt verification must be zero-write");

    let mut unknown: serde_json::Value = serde_json::from_slice(&result_bytes).unwrap();
    unknown["proof_receipt"] = serde_json::json!("PASS");
    assert!(WorkerResultV1::parse_json(&serde_json::to_vec(&unknown).unwrap()).is_err());

    let mut stale_candidate = result.clone();
    stale_candidate.candidate_identity.insert(
        "candidate_id".to_owned(),
        serde_json::json!(OTHER_CANDIDATE_ID),
    );
    assert!(stale_candidate.validate_for(&lease, &package).is_err());

    let mut missing_artifact = result.clone();
    missing_artifact.artifacts.remove(0);
    assert!(
        !r3_artifact_set_is_exact(&missing_artifact),
        "composed exact-set validation rejects a missing artifact row"
    );

    let mut replaced_artifact = result.clone();
    replaced_artifact.artifacts[0].path = R3_RESULT_PATH.to_owned();
    assert!(
        !r3_artifact_set_is_exact(&replaced_artifact),
        "composed exact-set validation rejects an allowed-path substitution"
    );

    let mut substituted_artifact = result.clone();
    substituted_artifact.artifacts[0].sha256 = sha(b"substituted artifact receipt");
    assert!(
        workspace
            .verify(&substituted_artifact, &lease, &package)
            .is_err()
    );

    let mut substituted_fixture = result.clone();
    let fixture = substituted_fixture
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.path == "fixtures/routine-production-catalog/cases.json")
        .unwrap();
    fixture.sha256 = sha(b"substituted fixture receipt");
    assert!(
        workspace
            .verify(&substituted_fixture, &lease, &package)
            .is_err()
    );
    eprintln!("ROUTINE_PRODUCTION_CATALOG_R3_RESULT_ID={result_id}");
}
