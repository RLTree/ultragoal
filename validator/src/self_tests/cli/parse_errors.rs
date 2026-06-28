fn args(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|arg| (*arg).to_string()).collect()
}

fn transactional_receipt(command: crate::Command) -> Option<std::path::PathBuf> {
    match command {
        crate::Command::TransactionalFinalization { receipt } => Some(receipt),
        _ => None,
    }
}

#[test]
fn process_entrypoint_fails_closed_for_test_harness_arguments() {
    assert_eq!(crate::main_entry(), 2);
}

#[test]
fn review_round_parse_requires_review_target_receipt_argument() {
    let err = crate::parse_command(&args(&[
        "review-round",
        "verify",
        "--receipt",
        "round.json",
        "--validator-receipt",
        "validator.json",
        "--archive-receipt",
        "archive.json",
    ]))
    .expect_err("missing review target receipt");
    assert!(err.contains("missing required argument --review-target-receipt"));
}

#[test]
fn transaction_finalize_parse_constructs_typed_receipt_command() {
    let command = crate::parse_command(&args(&["transaction", "finalize", "--receipt", "tx.json"]))
        .expect("transaction finalize command");
    assert_eq!(
        transactional_receipt(command),
        Some(std::path::PathBuf::from("tx.json"))
    );
    assert_eq!(transactional_receipt(crate::Command::PackageDigest), None);
}

#[test]
fn parser_error_arms_are_explicit_for_required_receipts() {
    let cases = [
        (
            &["audit"][..],
            "missing required argument --receipt",
            "audit receipt",
        ),
        (
            &[
                "review-round",
                "verify",
                "--validator-receipt",
                "validator.json",
                "--review-target-receipt",
                "target.json",
                "--archive-receipt",
                "archive.json",
            ][..],
            "missing required argument --receipt",
            "review round receipt",
        ),
        (
            &[
                "review-round",
                "verify",
                "--receipt",
                "round.json",
                "--validator-receipt",
                "validator.json",
                "--review-target-receipt",
                "target.json",
            ][..],
            "missing required argument --archive-receipt",
            "review round archive",
        ),
    ];

    for (raw, expected, label) in cases {
        let err = crate::parse_command(&args(raw)).expect_err(label);
        assert!(
            err.contains(expected),
            "{label}: expected {expected:?}, got {err:?}"
        );
    }
}

#[test]
fn parser_error_arms_are_explicit_for_fallback_command_families() {
    let cases = [
        (
            &["performance", "prove", "--class", "not-a-budget"][..],
            "invalid --class performance budget",
            "performance class",
        ),
        (
            &["rust", "unknown"][..],
            "unknown ultragoal rust command",
            "rust command",
        ),
        (
            &["gc", "unknown"][..],
            "unknown ultragoal gc command",
            "gc command",
        ),
    ];

    for (raw, expected, label) in cases {
        let err = crate::parse_command(&args(raw)).expect_err(label);
        assert!(
            err.contains(expected),
            "{label}: expected {expected:?}, got {err:?}"
        );
    }
}
