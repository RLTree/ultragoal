pub(super) fn is_help_request(raw: &[String]) -> bool {
    let is_help = |arg: &str| matches!(arg, "help" | "--help" | "-h");
    raw.iter()
        .enumerate()
        .any(|(index, arg)| is_help(arg) && !is_option_value(raw, index))
}

fn is_option_value(raw: &[String], index: usize) -> bool {
    index > 0 && option_accepts_value(&raw[index - 1])
}

fn option_accepts_value(arg: &str) -> bool {
    matches!(
        arg,
        "--app-path"
            | "--apply-receipt-digest"
            | "--archive-purpose"
            | "--archive-receipt"
            | "--budget-class"
            | "--byte-limit"
            | "--cache-mode"
            | "--cache-root"
            | "--call-receipt"
            | "--candidate"
            | "--check-id"
            | "--claim-id"
            | "--class"
            | "--classifier-actor-id"
            | "--command"
            | "--contract-id"
            | "--contract-version"
            | "--correlation-id"
            | "--endpoint"
            | "--family"
            | "--file"
            | "--implementation-kind"
            | "--input"
            | "--input-digest"
            | "--installed-root"
            | "--jobs"
            | "--law"
            | "--law-id"
            | "--limit"
            | "--mode"
            | "--model"
            | "--node"
            | "--obligation"
            | "--observability-receipt"
            | "--out-dir"
            | "--output-digest"
            | "--parsed-output-digest"
            | "--parser-schema-id"
            | "--plan-digest"
            | "--policy"
            | "--producer-actor-id"
            | "--profile"
            | "--prompt-contract-digest"
            | "--promptfoo-bin"
            | "--provider"
            | "--provider-policy"
            | "--purpose"
            | "--query"
            | "--receipt"
            | "--receipt-dir"
            | "--red-report"
            | "--report"
            | "--root"
            | "--run-id"
            | "--schema"
            | "--schema-id"
            | "--tier"
            | "--timeout-ms"
            | "--validator-receipt"
            | "--zip"
            | "--zip-root"
    )
}
