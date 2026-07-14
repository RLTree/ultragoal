fn current_inputs() -> (PathBuf, Vec<u8>, IssuedCurrentSubject) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let result_bytes = fs::read(root.join(CURRENT_RESULT)).unwrap();
    let subject = issued_current_subject(&root);
    (root, result_bytes, subject)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn verify_current_subject(
    root: &PathBuf,
    current: &WorkerResultV1,
    bound: &IssuedCurrentSubject,
    supplied: &IssuedCurrentSubject,
) -> Result<VerifiedArtifactSet, OrchestrationError> {
    if supplied != bound
        || current
            .base_state
            .get("work_envelope_sha256")
            .and_then(Value::as_str)
            != Some(bound.work_envelope_sha256.as_str())
    {
        return Err(OrchestrationError::StaleBinding);
    }
    current.validate_for(&supplied.lease, &supplied.package)?;
    ArtifactWorkspace::new(root)?.verify(current, &supplied.lease, &supplied.package)
}

#[test]
fn accepted_results_become_stale_when_their_bound_source_topology_changes() {
    let (root, bytes, lease, package) = historical_inputs();
    let historical = WorkerResultV1::parse_json(&bytes).unwrap();
    assert_eq!(historical.touched_paths, source_paths());
    assert_eq!(historical.artifacts.len(), source_paths().len());
    historical.validate_for(&lease, &package).unwrap();
    assert!(matches!(
        ArtifactWorkspace::new(&root)
            .unwrap()
            .verify(&historical, &lease, &package),
        Err(OrchestrationError::InvalidWorkerResult)
    ));

    let (root, bytes, bound) = current_inputs();
    let current = WorkerResultV1::parse_json(&bytes).unwrap();
    assert_eq!(current.touched_paths, current_paths());
    assert_eq!(current.artifacts.len(), current_paths().len());
    assert_eq!(
        current
            .base_state
            .get("agent_task_envelope_sha256")
            .and_then(Value::as_str),
        Some(digest(&fs::read(root.join(CURRENT_TASK)).unwrap()).as_str())
    );
    assert_eq!(
        current
            .base_state
            .get("work_envelope_sha256")
            .and_then(Value::as_str),
        Some(bound.work_envelope_sha256.as_str())
    );
    assert_eq!(
        verify_current_subject(&root, &current, &bound, &bound),
        Err(OrchestrationError::InvalidWorkerResult)
    );

    let mut missing_dependency_evidence = bound.clone();
    missing_dependency_evidence
        .lease
        .prerequisite_evidence
        .dependency_nodes
        .clear();
    assert_ne!(missing_dependency_evidence, bound);
    assert_eq!(
        verify_current_subject(&root, &current, &bound, &missing_dependency_evidence),
        Err(OrchestrationError::StaleBinding)
    );

    let mut missing_tool_evidence = bound.clone();
    missing_tool_evidence
        .lease
        .prerequisite_evidence
        .required_tools
        .clear();
    assert_ne!(missing_tool_evidence, bound);
    assert_eq!(
        verify_current_subject(&root, &current, &bound, &missing_tool_evidence),
        Err(OrchestrationError::StaleBinding)
    );

    let mut missing_prerequisite_evidence = bound.clone();
    missing_prerequisite_evidence
        .lease
        .prerequisite_evidence
        .prerequisites
        .clear();
    assert_ne!(missing_prerequisite_evidence, bound);
    assert_eq!(
        verify_current_subject(&root, &current, &bound, &missing_prerequisite_evidence),
        Err(OrchestrationError::StaleBinding)
    );

    let mut missing_acceptance = bound.clone();
    missing_acceptance.package.acceptance.clear();
    assert_ne!(missing_acceptance, bound);
    assert_eq!(
        verify_current_subject(&root, &current, &bound, &missing_acceptance),
        Err(OrchestrationError::StaleBinding)
    );
}

#[test]
fn worker_result_negative_mutations_fail_closed() {
    let (root, bytes, lease, package) = historical_inputs();
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
