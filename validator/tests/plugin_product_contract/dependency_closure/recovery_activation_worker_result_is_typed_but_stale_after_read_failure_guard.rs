#[test]
fn recovery_activation_worker_result_is_typed_but_stale_after_read_failure_guard() {
    let envelope_bytes = std::fs::read(root().join(LIFECYCLE_RECOVERY_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&envelope_bytes)),
        LIFECYCLE_RECOVERY_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&envelope_bytes).unwrap();
    envelope.work_package.validate().unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_plan_authorization_engineer"
    );
    assert_eq!(envelope.no_claim_statement, WORKER_NO_CLAIM);

    let result = WorkerResultV1::parse_json(
        &std::fs::read(root().join(LIFECYCLE_RECOVERY_RESULT_PATH)).unwrap(),
    )
    .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    assert!(
        ArtifactWorkspace::new(root())
            .unwrap()
            .verify(&result, &envelope.lease, &envelope.work_package)
            .is_err(),
        "recovery activation must stale when read-failure source and tests change"
    );
    assert_eq!(result.no_claim_statement, WORKER_NO_CLAIM);
    assert_eq!(
        result.base_state["work_envelope_sha256"],
        LIFECYCLE_RECOVERY_ENVELOPE_SHA256
    );
    assert_eq!(
        result.final_state["status"],
        "candidate_for_root_acceptance"
    );
}

#[test]
fn read_failure_guard_worker_result_is_lease_bound_and_verifies_every_artifact() {
    let envelope_bytes = std::fs::read(root().join(LIFECYCLE_READ_FAILURE_ENVELOPE_PATH)).unwrap();
    assert_eq!(
        format!("sha256:{}", digest(&envelope_bytes)),
        LIFECYCLE_READ_FAILURE_ENVELOPE_SHA256
    );
    let envelope: LifecycleRootEnvelope = serde_json::from_slice(&envelope_bytes).unwrap();
    envelope.work_package.validate().unwrap();
    assert_eq!(
        envelope.lease.owner.as_str(),
        "/root/plugin_plan_authorization_engineer"
    );
    assert_eq!(envelope.no_claim_statement, WORKER_NO_CLAIM);

    let result = WorkerResultV1::parse_json(
        &std::fs::read(root().join(LIFECYCLE_READ_FAILURE_RESULT_PATH)).unwrap(),
    )
    .unwrap();
    result
        .validate_for(&envelope.lease, &envelope.work_package)
        .unwrap();
    let verified = ArtifactWorkspace::new(root())
        .unwrap()
        .verify(&result, &envelope.lease, &envelope.work_package)
        .unwrap();
    assert_eq!(verified.artifact_count(), 4);
    assert_eq!(result.no_claim_statement, WORKER_NO_CLAIM);
    assert_eq!(
        result.base_state["work_envelope_sha256"],
        LIFECYCLE_READ_FAILURE_ENVELOPE_SHA256
    );
    assert_eq!(
        result.final_state["status"],
        "candidate_for_root_acceptance"
    );
    assert_eq!(result.final_state["read_effect_failure_guard"], true);
    assert_eq!(
        result.final_state["read_only_failure_authority_closed"],
        true
    );
    assert_eq!(result.final_state["restore_calls_after_failure"], 0);
    assert_eq!(result.final_state["observe_calls_after_failure"], 0);
    assert_eq!(result.final_state["recursive_zero_write"], true);

    let artifact_paths = result
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        artifact_paths,
        BTreeSet::from([
            "validator/src/plugin_product/lifecycle/execution.rs",
            "validator/src/plugin_product/lifecycle/model.rs",
            "validator/tests/plugin_product_contract/dependency_closure.rs",
            "validator/tests/plugin_product_contract/lifecycle_contract.rs",
        ])
    );
}
