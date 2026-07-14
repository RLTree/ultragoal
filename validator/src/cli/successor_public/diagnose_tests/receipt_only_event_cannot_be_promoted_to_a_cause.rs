use super::*;

#[test]
pub(crate) fn receipt_only_event_cannot_be_promoted_to_a_cause() {
    let repository = Repository::new("diagnose-receipt-only");
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
pub(crate) fn complete_corrupt_store_is_reported_as_corruption_and_never_repaired_by_diagnose() {
    let repository = Repository::new("diagnose-corrupt");
    let context = read_context(&repository.root).unwrap();
    let state = current_state(&context);
    let finding = state.findings().first().expect("fixture has findings");
    let spool = repository
        .root
        .join("validation_artifacts/observability/spool");
    fs::create_dir_all(&spool).unwrap();
    let path = super::super::observe::store_path(&repository.root);
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
        value["observability"]["read_failure"]["class"],
        "corruption"
    );
    assert_eq!(value["observability"]["read_failure"]["stage"], "query");
    assert_eq!(
        value["observability"]["explanation"]["classification"],
        "corruption"
    );
    assert_eq!(tree(&repository.root), before_tree);
    assert_eq!(repository.status(), before_status);
}

#[test]
pub(crate) fn unselected_diagnosis_does_not_infer_event_causality() {
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

pub(crate) fn current_state(context: &crate::context::LiveContext) -> ProductState {
    let inventory = InventoryBuilder::new(context).build().unwrap();
    derive_adopted(context, &inventory).unwrap()
}

pub(crate) fn diagnose(repository: &Repository, finding: Option<&str>) -> Value {
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
