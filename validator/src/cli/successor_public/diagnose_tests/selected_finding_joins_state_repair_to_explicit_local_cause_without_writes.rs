use super::*;

#[test]
pub(crate) fn selected_finding_joins_state_repair_to_explicit_local_cause_without_writes() {
    let repository = Repository::new("diagnose-causal");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let store = EventStore::for_context(
        super::super::observe::store_path(&repository.root),
        &context,
        "successor-runtime",
    )
    .unwrap();
    let root_event = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "diagnose-causal-root",
        1,
        1,
        "inventory.scan",
        "fail",
    )
    .unwrap();
    let mut target = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "diagnose-causal-target",
        2,
        2,
        "inventory.reconcile",
        "fail",
    )
    .unwrap();
    target.set_parent(root_event.event_id()).unwrap();
    target.add_finding_ref(&finding.finding_id).unwrap();
    target.add_repair_ref(&finding.repair.repair_id).unwrap();
    assert!(store.append(&root_event).unwrap());
    assert!(store.append(&target).unwrap());
    context.revalidate().unwrap();

    let before_tree = tree(&repository.root);
    let before_status = repository.status();
    let value = diagnose(&repository, Some(&finding.finding_id));

    assert_eq!(value["schema_version"], "ProductStateDiagnose-v1");
    assert_eq!(value["findings"][0]["finding_id"], finding.finding_id);
    assert_eq!(
        value["observability"]["schema_version"],
        "PublicCausalDiagnosis-v1"
    );
    assert_eq!(value["observability"]["store_status"], "available");
    assert_eq!(
        value["observability"]["matched_event_id"],
        "diagnose-causal-target"
    );
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "observed-cause"
    );
    assert_eq!(
        value["observability"]["explanation"]["causal_event_ids"],
        serde_json::json!(["diagnose-causal-root", "diagnose-causal-target"])
    );
    assert_eq!(
        value["observability"]["failure_provenance"]["selection_rule"],
        "latest-bounded-causal-failure-then-latest-reference"
    );
    assert_eq!(
        value["observability"]["failure_provenance"]["selected"]["operation"],
        "inventory.reconcile"
    );
    assert_eq!(
        value["observability"]["failure_provenance"]["selected"]["outcome"],
        "fail"
    );
    assert_eq!(value["observability"]["claim_effect"], "none");
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
pub(crate) fn absent_store_withholds_cause_and_exposes_local_only_policy_without_writes() {
    let repository = Repository::new("diagnose-absent");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let before_tree = tree(&repository.root);
    let before_status = repository.status();

    let value = diagnose(&repository, Some(&finding.finding_id));

    assert_eq!(value["observability"]["store_status"], "absent");
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(
        value["observability"]["policy"]["external_export"],
        "disabled-safe-default-OD-004-OD-007"
    );
    assert_eq!(
        value["observability"]["policy"]["deletion"],
        "explicit-clear-api"
    );
    assert_eq!(value["observability"]["policy"]["mode"], "local-only");
    assert_eq!(
        value["observability"]["policy"]["configured_export_on_read"],
        "refused-no-external-effect"
    );
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
pub(crate) fn present_store_without_a_bound_event_withholds_cause_without_writes() {
    let repository = Repository::new("diagnose-no-match");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let store = EventStore::for_context(
        super::super::observe::store_path(&repository.root),
        &context,
        "successor-runtime",
    )
    .unwrap();
    let unrelated = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "diagnose-unrelated",
        1,
        1,
        "inventory.scan",
        "fail",
    )
    .unwrap();
    assert!(store.append(&unrelated).unwrap());
    context.revalidate().unwrap();
    let before_tree = tree(&repository.root);
    let before_status = repository.status();

    let value = diagnose(&repository, Some(&finding.finding_id));

    assert_eq!(value["observability"]["store_status"], "available");
    assert_eq!(value["observability"]["matched_event_id"], Value::Null);
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(
        value["observability"]["explanation"]["diagnostic_code"],
        "observe-evidence-missing:target-event"
    );
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}
