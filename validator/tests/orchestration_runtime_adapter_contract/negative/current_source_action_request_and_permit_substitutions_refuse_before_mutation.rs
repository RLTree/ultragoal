#[test]
fn current_source_action_request_and_permit_substitutions_refuse_before_mutation() {
    let (root, head, _) = interrupted_root("negative-resume-bindings");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head.clone(), 5);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let permit = permit_for_action(&authority, &action, inspection.tick);
    let exact = ResumeRequest {
        expected_head: head,
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        target: action.target.clone(),
    };
    let before = recursive_fingerprint(root.path());

    let mut wrong_tick = exact.clone();
    wrong_tick.tick += 1;
    let mut wrong_workers = exact.clone();
    wrong_workers.live_workers.clear();
    let mut wrong_head = exact.clone();
    wrong_head.expected_head.log_sha256 = digest('f');
    let mut wrong_target = exact.clone();
    wrong_target.target.lease_id = Some("lease-substitution".to_owned());
    let substitutions = [wrong_tick, wrong_workers, wrong_head, wrong_target];
    for request in substitutions {
        assert_eq!(
            adapter
                .execute_action(
                    RuntimeActionSource::Current(&view),
                    &action,
                    &authority,
                    &permit,
                    &RuntimeActionRequest::Resume(request),
                )
                .unwrap_err(),
            ProductError::AuthorityOperationMismatch
        );
        assert_eq!(recursive_fingerprint(root.path()), before);
    }

    let mut substituted_action = action.clone();
    substituted_action.snapshot_id = digest('e');
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &substituted_action,
                &authority,
                &permit,
                &RuntimeActionRequest::Resume(exact.clone()),
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let expired = issue_action_permit_for_test(
        &authority,
        RootActionPermitIssuance {
            operation: RootOperation::Resume,
            binding: action.authority_binding.clone(),
            workspace_identity: &action.workspace_identity,
            journal_head_identity: &action.journal_head_identity,
            issued_tick: 1,
            expires_tick: 2,
            nonce: b"runtime-adapter-expired-012345",
            target: action.target.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &expired,
                &RuntimeActionRequest::Resume(exact.clone()),
            )
            .unwrap_err(),
        ProductError::AuthorityExpired
    );

    let wrong_operation = issue_action_permit_for_test(
        &authority,
        RootActionPermitIssuance {
            operation: RootOperation::Recover,
            binding: action.authority_binding.clone(),
            workspace_identity: &action.workspace_identity,
            journal_head_identity: &action.journal_head_identity,
            issued_tick: 4,
            expires_tick: 10,
            nonce: b"runtime-adapter-wrong-op-012345",
            target: action.target.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &wrong_operation,
                &RuntimeActionRequest::Resume(exact.clone()),
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );

    let mut permit_target = action.target.clone();
    permit_target.result_commitment_id = Some(digest('f'));
    let wrong_target_permit = issue_action_permit_for_test(
        &authority,
        RootActionPermitIssuance {
            operation: RootOperation::Resume,
            binding: action.authority_binding.clone(),
            workspace_identity: &action.workspace_identity,
            journal_head_identity: &action.journal_head_identity,
            issued_tick: 4,
            expires_tick: 10,
            nonce: b"runtime-adapter-wrong-target-012345",
            target: permit_target,
        },
    )
    .unwrap();
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &wrong_target_permit,
                &RuntimeActionRequest::Resume(exact.clone()),
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let alternate_authority = root_authority_for_test(
        root_actor(),
        b"runtime-adapter-alternate-secret-0123456789abcdef",
    )
    .unwrap();
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &alternate_authority,
                &permit,
                &RuntimeActionRequest::Resume(exact.clone()),
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );

    let mut permit_value = serde_json::to_value(&permit).unwrap();
    permit_value["authenticator"] = serde_json::Value::String(digest('0'));
    let forged: RootPermit = serde_json::from_value(permit_value).unwrap();
    let error = adapter
        .execute_action(
            RuntimeActionSource::Current(&view),
            &action,
            &authority,
            &forged,
            &RuntimeActionRequest::Resume(exact),
        )
        .unwrap_err();
    assert_eq!(error, ProductError::AuthorityInvalid);
    assert!(!error.to_string().contains("authenticator"));
    assert_eq!(recursive_fingerprint(root.path()), before);
}
