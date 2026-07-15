use super::fixture::*;
use std::collections::BTreeSet;
use ultragoal::orchestration::product::command::OrchestrationStateRequest;
use ultragoal::orchestration::product::runtime_adapter::{
    OrchestrationRuntimeAdapter, RuntimeActionRequest, RuntimeActionSource,
};
use ultragoal::orchestration::product::{
    PermitReplayState, ProductError, ProductWorkspace, ProductionRootAuthority, ReconcileRequest,
    ResumeRequest, RootPermit,
};
use ultragoal::orchestration::{
    EffectClass, EffectGrant, EffectOutcome, EffectReceipt, EffectResolution,
};

#[test]
fn issuance_refuses_unbounded_lifetime_without_ledger_mutation() {
    let (journal, head) = interrupted_root("unbounded-lifetime-journal");
    let authority_root = TestRoot::new("unbounded-lifetime-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let before = recursive_fingerprint(authority_root.path());
    assert_eq!(
        adapter
            .issue_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                u64::MAX,
            )
            .unwrap_err(),
        ProductError::AuthorityInvalid
    );
    assert_eq!(recursive_fingerprint(authority_root.path()), before);
}

#[test]
fn exact_reconcile_permit_rejects_every_resolution_field_substitution() {
    let (journal, head) = ambiguous_effect("decision-binding-fields");
    let authority_root = TestRoot::new("decision-binding-authority", 0o700);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head.clone(),
            tick: 4,
            live_workers: BTreeSet::from(["worker-a".to_owned()]),
        })
        .unwrap();
    let action = view.state().root_action_requests[0].clone();
    let authority =
        ProductionRootAuthority::open_or_initialize(authority_root.path(), root_actor()).unwrap();
    let exact_resolution = applied_resolution();
    let permit = adapter
        .issue_production_reconcile(&authority, &view, &action, 14, &exact_resolution)
        .unwrap();
    let exact = request(&action, head, exact_resolution);
    let journal_before = recursive_fingerprint(journal.path());
    let authority_before = recursive_fingerprint(authority_root.path());

    for changed in resolution_substitutions(&exact) {
        assert_eq!(
            adapter
                .execute_production_reconcile(&authority, &view, &action, &permit, &changed)
                .unwrap_err(),
            ProductError::AuthorityInvalid
        );
        assert_eq!(recursive_fingerprint(journal.path()), journal_before);
        assert_eq!(
            recursive_fingerprint(authority_root.path()),
            authority_before
        );
        assert_eq!(
            authority.replay_state(&permit).unwrap(),
            Some(PermitReplayState::Issued)
        );
    }
    adapter
        .execute_production_reconcile(&authority, &view, &action, &permit, &exact)
        .unwrap();
}

#[test]
fn permit_downgrade_cross_binding_forgery_and_debug_leak_fail_closed() {
    let (journal, head) = interrupted_root("decision-binding-permit");
    let authority_root = TestRoot::new("decision-binding-permit-authority", 0o700);
    let (action, permit, authority) = issue_resume(&authority_root, &journal, head.clone(), 2);
    let context = context();
    let workspace = ProductWorkspace::open(journal.path()).unwrap();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace).unwrap();
    let view = adapter
        .inspect_current(&OrchestrationStateRequest {
            expected_head: head,
            tick: 2,
            live_workers: BTreeSet::new(),
        })
        .unwrap();
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: action.expected_head.clone(),
        tick: 2,
        live_workers: BTreeSet::new(),
        target: action.target.clone(),
    });
    let before = recursive_fingerprint(authority_root.path());
    let mut missing = serde_json::to_value(&permit).unwrap();
    missing.as_object_mut().unwrap().remove("decision_binding");
    assert!(serde_json::from_value::<RootPermit>(missing).is_err());

    for changed in permit_substitutions(&permit) {
        let error = adapter
            .execute_production_action(
                &authority,
                RuntimeActionSource::Current(&view),
                &action,
                &changed,
                &request,
            )
            .unwrap_err();
        assert_eq!(error, ProductError::AuthorityInvalid);
        assert!(!error.to_string().contains("authenticator"));
        assert_eq!(recursive_fingerprint(authority_root.path()), before);
    }
    let rendered = format!("{permit:?}");
    assert!(!rendered.contains(
        &serde_json::to_value(&permit).unwrap()["authenticator"]
            .as_str()
            .unwrap()
    ));
}

fn request(
    action: &ultragoal::orchestration::product::command::RootActionRequest,
    head: ultragoal::orchestration::JournalHead,
    resolution: EffectResolution,
) -> ReconcileRequest {
    ReconcileRequest {
        expected_head: head,
        tick: 4,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
        lease_id: "lease-001".to_owned(),
        resolution,
        target: action.target.clone(),
    }
}

fn applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('f'),
        outcome: EffectOutcome::Applied {
            receipt: EffectReceipt {
                operation_id: "operation-001".to_owned(),
                effect: effect(),
                receipt_digest: digest('d'),
            },
        },
    }
}

fn resolution_substitutions(exact: &ReconcileRequest) -> Vec<ReconcileRequest> {
    let mut evidence = exact.clone();
    evidence.resolution.evidence_digest = digest('1');
    let mut outcome = exact.clone();
    outcome.resolution.outcome = EffectOutcome::NotApplied;
    let mut operation = exact.clone();
    receipt(&mut operation).operation_id = "other-operation".to_owned();
    let mut effect_class = exact.clone();
    receipt(&mut effect_class).effect =
        EffectGrant::new(EffectClass::Network, "worker-effect").unwrap();
    let mut effect_target = exact.clone();
    receipt(&mut effect_target).effect =
        EffectGrant::new(EffectClass::Process, "other-effect").unwrap();
    let mut receipt_digest = exact.clone();
    receipt(&mut receipt_digest).receipt_digest = digest('2');
    vec![
        evidence,
        outcome,
        operation,
        effect_class,
        effect_target,
        receipt_digest,
    ]
}

fn receipt(request: &mut ReconcileRequest) -> &mut EffectReceipt {
    let EffectOutcome::Applied { receipt } = &mut request.resolution.outcome else {
        unreachable!()
    };
    receipt
}

fn permit_substitutions(permit: &RootPermit) -> Vec<RootPermit> {
    let original = serde_json::to_value(permit).unwrap();
    [
        ("schema_version", serde_json::json!("OrchestrationRootPermit-v1")),
        ("decision_binding", serde_json::json!({"kind":"reconcile_effect","effect_resolution_commitment_id":digest('7')})),
        ("authenticator", serde_json::json!(digest('0'))),
    ]
    .into_iter()
    .map(|(field, value)| {
        let mut changed = original.clone();
        changed[field] = value;
        serde_json::from_value(changed).unwrap()
    })
    .collect()
}
