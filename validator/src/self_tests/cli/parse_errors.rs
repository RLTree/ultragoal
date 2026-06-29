fn args(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|arg| (*arg).to_string()).collect()
}

fn transactional_receipt(command: crate::Command) -> Option<std::path::PathBuf> {
    match command {
        crate::Command::TransactionalFinalization { receipt } => Some(receipt),
        _ => None,
    }
}

fn final_packet_receipt(command: crate::Command) -> Option<std::path::PathBuf> {
    match command {
        crate::Command::FinalPacket(command) => Some(command.receipt),
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
fn final_packet_parse_constructs_typed_receipt_command() {
    let command = crate::parse_command(&args(&[
        "final-packet",
        "prove",
        "--receipt",
        "packet-proof.json",
    ]))
    .expect("final packet command");
    assert_eq!(
        final_packet_receipt(command),
        Some(std::path::PathBuf::from("packet-proof.json"))
    );
    let alias = crate::parse_command(&args(&["packet", "prove", "--receipt", "alias.json"]))
        .expect("packet alias command");
    assert_eq!(
        final_packet_receipt(alias),
        Some(std::path::PathBuf::from("alias.json"))
    );
    assert_eq!(final_packet_receipt(crate::Command::PackageDigest), None);
    assert!(
        crate::cli::final_packet::parse(&args(&["source", "audit"]))
            .expect("unrelated command parses")
            .is_none()
    );
    let err = crate::parse_command(&args(&["final-packet", "prove"])).expect_err("missing receipt");
    assert!(err.contains("missing required argument --receipt"));
    let usage = crate::parse_command(&args(&["packet", "unknown"])).expect_err("usage error");
    assert!(usage.contains("usage: ultragoal"));
}

#[test]
fn session_log_hardening_parse_constructs_typed_rebind_command() {
    let command = crate::parse_command(&args(&[
        "session-log",
        "hardening",
        "rebind",
        "--receipt",
        "validation_artifacts/harness/session-log-hardening-receipt.json",
    ]))
    .expect("session-log hardening command");
    assert!(matches!(
        command,
        crate::Command::Session(command)
            if command.receipt
                == std::path::PathBuf::from(
                    "validation_artifacts/harness/session-log-hardening-receipt.json"
                )
    ));
    let err = crate::parse_command(&args(&["session-log", "hardening", "rebind"]))
        .expect_err("missing receipt");
    assert!(err.contains("missing required argument --receipt"));
    let err = crate::parse_command(&args(&["session-log", "hardening"]))
        .expect_err("missing session subcommand");
    assert!(err.contains("unknown session-log command"));
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
