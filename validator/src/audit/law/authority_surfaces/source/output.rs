pub(super) fn failures_for_text(rel: &str, text: &str) -> Vec<String> {
    if rel == "validator/src/audit/law/authority_surfaces/source/output.rs" {
        return Vec::new();
    }
    if rel.contains("/self_tests/")
        || rel.contains("/tests/")
        || rel.ends_with("/tests.rs")
        || rel.ends_with("_tests.rs")
        || rel.contains("/test_")
    {
        return Vec::new();
    }
    let mut failures = Vec::new();
    for pattern in claim_output_patterns() {
        if text.lines().any(|line| output_pattern_line(line, pattern)) {
            failures.push(format!(
                "claim_artifact_output_without_typed_authority:path={rel};pattern={pattern};repair=use_output_path_claim_artifact_path_or_mark_external_debug_no_claim"
            ));
        }
    }
    let unsafe_vars = unsafe_claim_output_vars(text);
    for var in unsafe_vars {
        if text.lines().any(|line| writer_uses_var(line, &var)) {
            failures.push(format!(
                "claim_artifact_output_without_typed_authority:path={rel};var={var};repair=use_output_path_claim_artifact_path_or_mark_external_debug_no_claim"
            ));
        }
    }
    failures
}

fn claim_output_patterns() -> [&'static str; 24] {
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
        "std::path::PathBuf::from(",
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

fn unsafe_claim_output_vars(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('"') || trimmed.starts_with("//") {
            continue;
        }
        if !line_uses_raw_output_authority(trimmed) {
            continue;
        }
        if let Some(var) = assigned_var_name(trimmed)
            && !out.iter().any(|existing| existing == var)
        {
            out.push(var.to_string());
        }
    }
    out
}

fn line_uses_raw_output_authority(trimmed: &str) -> bool {
    (trimmed.contains("= root.join(")
        || trimmed.contains("= PathBuf::from(")
        || trimmed.contains("= std::path::PathBuf::from(")
        || trimmed.contains("= command.receipt.clone()")
        || trimmed.contains("= command.observability_receipt.clone()")
        || trimmed.contains("= command.receipt_rel()")
        || trimmed.contains("= receipt.clone()"))
        && !trimmed.contains("claim_artifact_path(")
        && !trimmed.contains("literal_claim_artifact_path(")
}

fn assigned_var_name(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("let ")?;
    let rest = rest.strip_prefix("mut ").unwrap_or(rest).trim_start();
    Some(
        rest.split(|ch: char| ch == ':' || ch == '=' || ch.is_whitespace())
            .next()
            .unwrap_or(""),
    )
}

fn writer_uses_var(line: &str, var: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('"') || trimmed.starts_with("//") {
        return false;
    }
    let borrowed = format!("write_json(&{var},");
    let direct = format!("write_json({var},");
    trimmed.contains(&borrowed) || trimmed.contains(&direct)
}
