use super::semantic_tokens::semantic_tokens;

pub(crate) fn product_opaque_goal_work_label(path: &str) -> Option<&'static str> {
    let tail = path.strip_prefix("validator/").unwrap_or(path);
    let lower = tail.to_ascii_lowercase();
    let compact = lower
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let tokens = semantic_tokens(tail);
    if let Some(label) = source_name_violation_label(&tokens, &compact) {
        return Some(label);
    }
    if lower.contains("production_proof")
        || lower.contains("production-proof")
        || compact == "productionproof"
        || adjacent_tokens(&tokens, "production", "proof")
    {
        return Some("production_proof");
    }
    if lower.contains("root_phase")
        || lower.contains("root-phase")
        || compact.contains("rootphase")
        || adjacent_tokens(&tokens, "root", "phase")
    {
        return Some("root_phase");
    }
    for (first, second, compact_name, label) in [
        ("fit", "command", "fitcommand", "fit_command"),
        ("fit", "path", "fitpath", "fit_path"),
        ("fit", "goal", "fitgoal", "fit_goal"),
        ("fit", "slice", "fitslice", "fit_slice"),
        ("phase4", "rebind", "phase4rebind", "phase4_rebind"),
        (
            "checkpoint",
            "progress",
            "checkpointprogress",
            "checkpoint_progress",
        ),
        ("todo", "repair", "todorepair", "todo_repair"),
    ] {
        if compact == compact_name || adjacent_tokens(&tokens, first, second) {
            return Some(label);
        }
    }
    for token in &tokens {
        match token.as_str() {
            "fitting" => return Some("fitting"),
            "slice" => return Some("slice"),
            "phase" => return Some("phase"),
            "workstream" => return Some("workstream"),
            "checkpoint" => return Some("checkpoint"),
            "progress" => return Some("progress"),
            "wip" => return Some("wip"),
            "todo" => return Some("todo"),
            "scratch" => return Some("scratch"),
            "productionproof" => return Some("production_proof"),
            "proofstatus" => return Some("proof_status"),
            "evidencestatus" => return Some("evidence_status"),
            _ => {}
        }
        if token
            .strip_prefix("gate")
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
        {
            return Some("gate_number");
        }
        if token
            .strip_prefix("phase")
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()))
        {
            return Some("phase_number");
        }
    }
    if adjacent_numbered_label(&tokens, "gate") {
        return Some("gate_number");
    }
    if adjacent_numbered_label(&tokens, "phase") {
        return Some("phase_number");
    }
    None
}

fn source_name_violation_label(tokens: &[String], compact: &str) -> Option<&'static str> {
    for (first, second, compact_name, label) in [
        ("proof", "status", "proofstatus", "proof_status"),
        ("proof", "state", "proofstate", "proof_state"),
        ("evidence", "status", "evidencestatus", "evidence_status"),
        ("evidence", "posture", "evidenceposture", "evidence_posture"),
        (
            "production",
            "evidence",
            "productionevidence",
            "production_evidence",
        ),
        ("validation", "proof", "validationproof", "validation_proof"),
    ] {
        if compact == compact_name || adjacent_tokens(tokens, first, second) {
            return Some(label);
        }
    }
    None
}

pub(crate) fn generic_identifier_bucket_label(identifier: &str) -> Option<&'static str> {
    let lower = identifier.to_ascii_lowercase();
    let tokens = semantic_tokens(identifier);
    if lower == "support" || tokens.iter().any(|token| token == "support") {
        return Some("support");
    }
    if matches!(lower.as_str(), "helper" | "helpers")
        || tokens
            .iter()
            .any(|token| matches!(token.as_str(), "helper" | "helpers"))
    {
        return Some("helper");
    }
    if matches!(lower.as_str(), "utils" | "utility" | "utilities")
        || tokens
            .iter()
            .any(|token| matches!(token.as_str(), "utils" | "utility" | "utilities"))
    {
        return Some("utils");
    }
    if lower == "common"
        || lower.starts_with("common_")
        || lower.ends_with("_common")
        || tokens.iter().any(|token| token == "common")
    {
        return Some("common");
    }
    if lower == "shared" {
        return Some("shared");
    }
    if lower == "misc" {
        return Some("misc");
    }
    None
}

pub(crate) fn product_opaque_goal_work_string_label(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    let compact = lower
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let tokens = semantic_tokens(text);
    if let Some(label) = source_name_violation_label(&tokens, &compact) {
        return Some(label);
    }
    if lower.contains("production_proof")
        || lower.contains("production-proof")
        || lower.contains("production proof")
        || compact.contains("productionproof")
        || adjacent_tokens(&tokens, "production", "proof")
    {
        return Some("production_proof");
    }
    if lower.contains("root_phase")
        || lower.contains("root-phase")
        || lower.contains("root phase")
        || compact.contains("rootphase")
        || adjacent_tokens(&tokens, "root", "phase")
    {
        return Some("root_phase");
    }
    if tokens.iter().any(|token| token == "fitting") {
        return Some("fitting");
    }
    for (first, second, compact_name, label) in [
        ("fit", "command", "fitcommand", "fit_command"),
        ("fit", "path", "fitpath", "fit_path"),
        ("fit", "goal", "fitgoal", "fit_goal"),
        ("fit", "slice", "fitslice", "fit_slice"),
        ("phase4", "rebind", "phase4rebind", "phase4_rebind"),
        (
            "checkpoint",
            "progress",
            "checkpointprogress",
            "checkpoint_progress",
        ),
        ("todo", "repair", "todorepair", "todo_repair"),
    ] {
        if compact.contains(compact_name) || adjacent_tokens(&tokens, first, second) {
            return Some(label);
        }
    }
    None
}

pub(crate) fn generic_source_leaf_label(stem: &str) -> Option<&'static str> {
    let tokens = semantic_tokens(stem);
    if tokens.iter().any(|token| token == "support") {
        return Some("support");
    }
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "helper" | "helpers"))
    {
        return Some("helper");
    }
    if tokens
        .iter()
        .any(|token| matches!(token.as_str(), "utils" | "utility" | "utilities"))
    {
        return Some("utils");
    }
    if tokens.iter().any(|token| token == "common") {
        return Some("common");
    }
    if stem == "shared" {
        return Some("shared");
    }
    if stem == "misc" {
        return Some("misc");
    }
    if stem == "nodes" {
        return Some("nodes");
    }
    None
}

fn adjacent_tokens(tokens: &[String], first: &str, second: &str) -> bool {
    tokens.windows(2).any(|window| {
        window.first().is_some_and(|token| token == first)
            && window.get(1).is_some_and(|token| token == second)
    })
}

fn adjacent_numbered_label(tokens: &[String], label: &str) -> bool {
    tokens.windows(2).any(|window| {
        window.first().is_some_and(|token| token == label)
            && window
                .get(1)
                .is_some_and(|token| token.chars().all(|ch| ch.is_ascii_digit()))
    })
}
