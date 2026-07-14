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
