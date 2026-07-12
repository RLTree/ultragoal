use super::test_support::{Repository, tree};
use super::{execute_invocation, read_context};
use crate::cli::successor::{OutputMode, ParseOutcome, parse_args};
use crate::inventory::InventoryBuilder;
use crate::observability::{EventStore, SemanticEvent};
use crate::state::{ProductState, derive_adopted};
use serde_json::Value;
use std::fs;

#[test]
fn selected_finding_joins_state_repair_to_explicit_local_cause_without_writes() {
    let repository = Repository::new("diagnose-causal");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let store = EventStore::for_context(
        super::observe::store_path(&repository.root),
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
    assert_eq!(value["observability"]["claim_effect"], "none");
    assert_eq!(value["claim_effect"], "none");
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
fn absent_store_withholds_cause_and_exposes_local_only_policy_without_writes() {
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
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
fn present_store_without_a_bound_event_withholds_cause_without_writes() {
    let repository = Repository::new("diagnose-no-match");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let store = EventStore::for_context(
        super::observe::store_path(&repository.root),
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

#[test]
fn receipt_only_event_cannot_be_promoted_to_a_cause() {
    let repository = Repository::new("diagnose-receipt-only");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let store = EventStore::for_context(
        super::observe::store_path(&repository.root),
        &context,
        "successor-runtime",
    )
    .unwrap();
    let mut receipt = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "diagnose-receipt-only",
        1,
        1,
        "receipt.command",
        "fail",
    )
    .unwrap();
    receipt.add_finding_ref(&finding.finding_id).unwrap();
    receipt.add_repair_ref(&finding.repair.repair_id).unwrap();
    assert!(store.append(&receipt).unwrap());
    context.revalidate().unwrap();

    let value = diagnose(&repository, Some(&finding.finding_id));

    assert_eq!(
        value["observability"]["matched_event_id"],
        "diagnose-receipt-only"
    );
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "missing-evidence"
    );
    assert_eq!(
        value["observability"]["explanation"]["diagnostic_code"],
        "observe-evidence-missing:receipt-only"
    );
    assert_eq!(value["claim_effect"], "none");
}

#[test]
fn complete_corrupt_store_is_reported_as_corruption_and_never_repaired_by_diagnose() {
    let repository = Repository::new("diagnose-corrupt");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let path = super::observe::store_path(&repository.root);
    let store = EventStore::for_context(&path, &context, "successor-runtime").unwrap();
    let event = SemanticEvent::for_context(
        &context,
        "successor-runtime",
        "diagnose-corrupt-seed",
        1,
        1,
        "inventory.scan",
        "fail",
    )
    .unwrap();
    assert!(store.append(&event).unwrap());
    fs::write(&path, b"{\"complete\":\"corrupt\"}\n").unwrap();
    context.revalidate().unwrap();
    let before_tree = tree(&repository.root);
    let before_status = repository.status();

    let value = diagnose(&repository, Some(&finding.finding_id));

    assert_eq!(value["observability"]["store_status"], "available");
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "corruption"
    );
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
fn unselected_diagnosis_does_not_infer_event_causality() {
    let repository = Repository::new("diagnose-unselected");
    let before_tree = tree(&repository.root);
    let before_status = repository.status();

    let value = diagnose(&repository, None);

    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "not-evaluated"
    );
    assert_eq!(value["observability"]["store_status"], "not_opened");
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

fn current_state(context: &crate::context::LiveContext) -> ProductState {
    let inventory = InventoryBuilder::new(context).build().unwrap();
    derive_adopted(context, &inventory).unwrap()
}

fn diagnose(repository: &Repository, finding: Option<&str>) -> Value {
    let args = match finding {
        Some(finding) => vec!["--json", "diagnose", "--finding", finding],
        None => vec!["--json", "diagnose"],
    };
    let ParseOutcome::Invocation(invocation) = parse_args(args).unwrap() else {
        panic!("expected invocation")
    };
    let streams = execute_invocation(&repository.root, invocation).render(OutputMode::Json);
    assert_eq!(streams.exit_code, 1);
    assert!(streams.stderr.is_empty());
    serde_json::from_slice(&streams.stdout).unwrap()
}
