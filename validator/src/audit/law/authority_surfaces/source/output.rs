pub(super) fn failures_for_text(rel: &str, text: &str) -> Vec<String> {
    if rel.contains("/self_tests/")
        || rel.contains("/tests/")
        || rel.ends_with("/tests.rs")
        || rel.contains("/test_")
    {
        return Vec::new();
    }
    claim_output_patterns()
        .into_iter()
        .filter(|pattern| text.lines().any(|line| output_pattern_line(line, pattern)))
        .map(|pattern| {
            format!(
                "claim_artifact_output_without_typed_authority:path={rel};pattern={pattern};repair=use_output_path_claim_artifact_path_or_mark_external_debug_no_claim"
            )
        })
        .collect()
}

fn claim_output_patterns() -> [&'static str; 23] {
    [
        "command.receipt.clone()",
        "command.observability_receipt.clone()",
        "resolve(root, &command.receipt)",
        "root.join(&command.receipt)",
        "root.join(&command.observability_receipt)",
        "root.join(&observability_receipt)",
        "root.join(command.receipt_rel())",
        "root.join(receipt)",
        "root.join(RECEIPT_REL)",
        "root.join(OBSERVABILITY_RECEIPT_REL)",
        "root.join(observability_path)",
        "root.join(fields.receipt_path)",
        "root.join(ACTIVE_RECEIPT)",
        "root.join(RAW_OBSERVATION)",
        "PathBuf::from(&command.receipt)",
        "PathBuf::from(&command.observability_receipt)",
        "PathBuf::from(receipt)",
        "json_boundary::write_json(receipt,",
        "json_boundary::write_json(path,",
        "json_boundary::write_json(&command.receipt,",
        "json_boundary::write_json(&root.join(&observability_receipt)",
        "json_boundary::write_json(&root.join(command.receipt_rel())",
        "json_boundary::write_json(&root.join(OBSERVABILITY_RECEIPT_REL)",
    ]
}

fn output_pattern_line(line: &str, pattern: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('"') || trimmed.starts_with("//") {
        return false;
    }
    line.contains(pattern)
}
