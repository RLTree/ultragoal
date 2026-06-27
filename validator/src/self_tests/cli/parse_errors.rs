fn args(raw: &[&str]) -> Vec<String> {
    raw.iter().map(|arg| (*arg).to_string()).collect()
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
