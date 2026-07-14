use super::runtime_fixture::*;
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;

fn reconcile_request(
    inspection: &crate::orchestration::product::command::OrchestrationStateRequest,
    action: &crate::orchestration::product::command::RootActionRequest,
    resolution: EffectResolution,
) -> ReconcileRequest {
    ReconcileRequest {
        expected_head: inspection.expected_head.clone(),
        tick: inspection.tick,
        live_workers: inspection.live_workers.clone(),
        lease_id: "lease-001".to_owned(),
        resolution,
        target: action.target.clone(),
    }
}

#[test]
fn exact_resolution_permit_rejects_evidence_outcome_and_every_receipt_field_substitution() {
    let (root, head) = ambiguous_effect("decision-binding-substitution");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head, 4);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let exact = reconcile_request(&inspection, &action, applied_resolution());
    let permit = permit_for_reconciliation(&authority, &action, inspection.tick, &exact.resolution);
    let before = recursive_fingerprint(root.path());

    let mut evidence = exact.clone();
    evidence.resolution.evidence_digest = digest('1');
    let mut outcome = exact.clone();
    outcome.resolution.outcome = EffectOutcome::NotApplied;
    let mut receipt_operation = exact.clone();
    let EffectOutcome::Applied { receipt } = &mut receipt_operation.resolution.outcome else {
        unreachable!()
    };
    receipt.operation_id = "operation-substitution".to_owned();
    let mut receipt_effect_class = exact.clone();
    let EffectOutcome::Applied { receipt } = &mut receipt_effect_class.resolution.outcome else {
        unreachable!()
    };
    receipt.effect = EffectGrant::new(EffectClass::Network, "worker-effect").unwrap();
    let mut receipt_effect_target = exact.clone();
    let EffectOutcome::Applied { receipt } = &mut receipt_effect_target.resolution.outcome else {
        unreachable!()
    };
    receipt.effect = EffectGrant::new(EffectClass::Process, "substituted-effect").unwrap();
    let mut receipt_digest = exact.clone();
    let EffectOutcome::Applied { receipt } = &mut receipt_digest.resolution.outcome else {
        unreachable!()
    };
    receipt.receipt_digest = digest('2');

    for request in [
        evidence,
        outcome,
        receipt_operation,
        receipt_effect_class,
        receipt_effect_target,
        receipt_digest,
    ] {
        assert_eq!(
            adapter
                .execute_reconcile(&view, &action, &authority, &permit, &request)
                .unwrap_err(),
            ProductError::AuthorityInvalid
        );
        assert_eq!(recursive_fingerprint(root.path()), before);
    }

    let outcome = adapter
        .execute_reconcile(&view, &action, &authority, &permit, &exact)
        .unwrap();
    assert_eq!(outcome.settled_operation_id, exact.resolution.operation_id);
    let after = recursive_fingerprint(root.path());
    assert_eq!(
        adapter
            .execute_reconcile(&view, &action, &authority, &permit, &exact)
            .unwrap_err(),
        ProductError::ConcurrentUpdate
    );
    assert_eq!(recursive_fingerprint(root.path()), after);
}

#[test]
fn v1_missing_or_mismatched_decision_bindings_fail_closed_without_writes() {
    let (root, head, _) = interrupted_root("decision-binding-downgrade");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head, 5);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let permit = permit_for_action(&authority, &action, inspection.tick);
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: inspection.expected_head,
        tick: inspection.tick,
        live_workers: inspection.live_workers,
        target: action.target.clone(),
    });
    let before = recursive_fingerprint(root.path());

    let mut missing = serde_json::to_value(&permit).unwrap();
    missing.as_object_mut().unwrap().remove("decision_binding");
    assert!(serde_json::from_value::<RootPermit>(missing).is_err());
    assert_eq!(recursive_fingerprint(root.path()), before);

    let mut downgraded = serde_json::to_value(&permit).unwrap();
    downgraded["schema_version"] = serde_json::Value::String("OrchestrationRootPermit-v1".into());
    let downgraded: RootPermit = serde_json::from_value(downgraded).unwrap();
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &downgraded,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);

    let mut cross_bound = serde_json::to_value(&permit).unwrap();
    cross_bound["decision_binding"] = serde_json::json!({
        "kind": "reconcile_effect",
        "effect_resolution_commitment_id": digest('7')
    });
    let cross_bound: RootPermit = serde_json::from_value(cross_bound).unwrap();
    assert_eq!(
        adapter
            .execute_action(
                RuntimeActionSource::Current(&view),
                &action,
                &authority,
                &cross_bound,
                &request,
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn action_and_reconciliation_permits_cannot_cross_interfaces() {
    let (root, head) = ambiguous_effect("decision-binding-cross-use");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head, 4);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let request = reconcile_request(&inspection, &action, not_applied_resolution());
    let reconcile_permit =
        permit_for_reconciliation(&authority, &action, inspection.tick, &request.resolution);
    let action_permit = issue_action_permit_for_test(
        &authority,
        RootActionPermitIssuance {
            operation: RootOperation::Resume,
            binding: action.authority_binding.clone(),
            workspace_identity: &action.workspace_identity,
            journal_head_identity: &action.journal_head_identity,
            issued_tick: 3,
            expires_tick: 10,
            nonce: b"runtime-cross-use-action-012345",
            target: action.target.clone(),
        },
    )
    .unwrap();
    let before = recursive_fingerprint(root.path());

    assert_eq!(
        adapter
            .execute_reconcile(&view, &action, &authority, &action_permit, &request)
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(root.path()), before);

    let action_shape = RuntimeActionRequest::Resume(ResumeRequest {
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
                &reconcile_permit,
                &action_shape,
            )
            .unwrap_err(),
        ProductError::AuthorityOperationMismatch
    );
    assert_eq!(recursive_fingerprint(root.path()), before);
}

#[test]
fn permit_debug_and_refusal_diagnostics_do_not_echo_authority_material() {
    let (root, head, _) = interrupted_root("decision-binding-non-echo");
    let context = context();
    let workspace = ProductWorkspace::open(root.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let inspection = state_request(head, 5);
    let view = adapter.inspect_current(&inspection).unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority = authority();
    let permit = permit_for_action(&authority, &action, inspection.tick);
    let serialized = serde_json::to_value(&permit).unwrap();
    let authenticator = serialized["authenticator"].as_str().unwrap().to_owned();
    let debug = format!("{permit:?}");
    assert!(!debug.contains(&authenticator));
    assert!(!debug.contains(std::str::from_utf8(ROOT_SECRET).unwrap()));

    let mut forged = serialized;
    forged["authenticator"] = serde_json::Value::String(digest('0'));
    let forged: RootPermit = serde_json::from_value(forged).unwrap();
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: inspection.expected_head,
        tick: inspection.tick,
        live_workers: inspection.live_workers,
        target: action.target.clone(),
    });
    let before = recursive_fingerprint(root.path());
    let error = adapter
        .execute_action(
            RuntimeActionSource::Current(&view),
            &action,
            &authority,
            &forged,
            &request,
        )
        .unwrap_err();
    assert!(!error.to_string().contains(&authenticator));
    assert!(!error.to_string().contains("runtime-adapter-root-secret"));
    assert_eq!(recursive_fingerprint(root.path()), before);
}
