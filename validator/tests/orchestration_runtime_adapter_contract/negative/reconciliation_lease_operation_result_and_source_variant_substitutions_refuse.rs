#[test]
fn reconciliation_lease_operation_result_and_source_variant_substitutions_refuse() {
    let (root, head) = ambiguous_effect("negative-reconcile-bindings");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head, 4);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let exact = ReconcileRequest {
        expected_head: inspection.expected_head.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        lease_id: "lease-001".to_owned(),
        resolution: not_applied_resolution(),
        target: action.target.clone(),
    };
    let permit = permit_for_reconciliation(&authority, &action, inspection.tick, &exact.resolution);
    let before = recursive_fingerprint(root.path());

    let mut wrong_lease = exact.clone();
    wrong_lease.lease_id = "lease-substitution".to_owned();
    let mut wrong_operation = exact.clone();
    wrong_operation.resolution.operation_id = "operation-substitution".to_owned();
    let mut wrong_result = exact.clone();
    wrong_result.target.result_commitment_id = Some(digest('f'));
    for request in [wrong_lease, wrong_operation, wrong_result] {
        assert_eq!(
            adapter
                .execute_reconcile(&view, &action, &authority, &permit, &request,)
                .unwrap_err(),
            ProductError::AuthorityOperationMismatch
        );
        assert_eq!(recursive_fingerprint(root.path()), before);
    }

    let resume_shape = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: inspection.expected_head,
        tick: inspection.tick,
        live_workers: inspection.live_workers,
        target: action.target.clone(),
    });
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &permit,
                &resume_shape,
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn interrupted_request_event_binding_tick_workers_and_source_substitutions_refuse() {
    let (root, _, _, inspection) = interrupted_heartbeat("negative-recover-bindings");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter.inspect_interrupted(&inspection).unwrap();
    let action = view.preview().root_action_request.clone();
    let authority = authority();
    let permit = permit_for_action(&authority, &action, inspection.tick);
    let exact = RecoverRequest {
        expected_prior_head: inspection.expected_prior_head.clone(),
        expected_event_id: inspection.expected_event_id.clone(),
        recovered_binding: inspection.recovered_binding.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        target: action.target.clone(),
    };
    let before = recursive_fingerprint(root.path());

    let mut wrong_event = exact.clone();
    wrong_event.expected_event_id = digest('f');
    let mut wrong_binding = exact.clone();
    wrong_binding.recovered_binding = alternate_binding();
    let mut wrong_tick = exact.clone();
    wrong_tick.tick += 1;
    let mut wrong_workers = exact.clone();
    wrong_workers.live_workers = BTreeSet::new();
    let mut wrong_target = exact.clone();
    wrong_target.target.operation_id = Some(digest('f'));
    for request in [
        wrong_event,
        wrong_binding,
        wrong_tick,
        wrong_workers,
        wrong_target,
    ] {
        assert_eq!(
            adapter
                .execute_action(
                    RuntimeActionSource::Interrupted(&view),
                    &action,
                    &authority,
                    &permit,
                    &RuntimeActionRequest::Recover(request),
                )
                .unwrap_err(),
            ProductError::AuthorityOperationMismatch
        );
        assert_eq!(recursive_fingerprint(root.path()), before);
    }

    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn stale_view_and_candidate_context_refuse_without_additional_writes() {
    let (root, head, _) = interrupted_root("negative-stale-view");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head.clone(), 5);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let permit = permit_for_action(&authority, &action, inspection.tick);
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: inspection.expected_head.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        target: action.target.clone(),
    });

    let mut writer = Orchestrator::restart_durable(
        graph(),
        policy(),
        head,
        root_actor(),
        root.path(),
        TestSink { fail: false },
    )
    .unwrap();
    writer.recover_root(5).unwrap();
    drop(writer);
    let after_external_write = recursive_fingerprint(root.path());
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &permit,
                &request,
            )
            .unwrap_err(),
        ProductError::ConcurrentUpdate
    );
    assert_eq!(recursive_fingerprint(root.path()), after_external_write);

    let stale_context = ProductContext::new(graph(), policy(), alternate_binding(), root_actor());
    let stale_adapter = OrchestrationRuntimeAdapter::new(&stale_context, &workspace).unwrap();
    let current = FileJournal::open(root.path())
        .unwrap()
        .inspect()
        .unwrap()
        .head;
    let before = recursive_fingerprint(root.path());
    assert_eq!(
        stale_adapter
            .inspect_current(&state_request(current, 5))
            .unwrap_err(),
        ProductError::StaleCandidate
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}
