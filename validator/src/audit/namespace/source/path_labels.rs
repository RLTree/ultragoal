use super::semantic_tokens::semantic_tokens;

const GOAL_WORK_PAIRS: &[(&str, &str, &str, &str)] = &[
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
];

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
    if let Some(label) = repo_entrypoint_context_label(&tokens) {
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
    for &(first, second, compact_name, label) in GOAL_WORK_PAIRS {
        if compact == compact_name || adjacent_tokens(&tokens, first, second) {
            return Some(label);
        }
    }
    if adjacent_numbered_label(&tokens, "gate") {
        return Some("gate_number");
    }
    if adjacent_numbered_label(&tokens, "phase") {
        return Some("phase_number");
    }
    if let Some(label) = standalone_goal_work_token_label(&tokens) {
        return Some(label);
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

fn repo_entrypoint_context_label(tokens: &[String]) -> Option<&'static str> {
    let entrypoint_index = tokens.iter().position(|token| token == "fit")?;
    let has_repo_neighbor = entrypoint_index
        .checked_sub(1)
        .and_then(|index| tokens.get(index))
        .is_some_and(|token| token == "repo")
        || tokens
            .get(entrypoint_index + 1)
            .is_some_and(|token| token == "repo");
    if has_repo_neighbor { None } else { Some("fit") }
}

pub(crate) fn generic_identifier_bucket_label(identifier: &str) -> Option<&'static str> {
    generic_bucket_label(identifier, false)
}

pub(crate) fn product_opaque_goal_work_string_label(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    let compact = lower
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let tokens = semantic_tokens(text);
    if let Some(label) = session_history_status_label(&lower) {
        return Some(label);
    }
    if !authority_like_string(text) {
        return None;
    }
    if let Some(label) = source_name_violation_label(&tokens, &compact) {
        return Some(label);
    }
    if lower.contains("production_proof")
        || lower.contains("production-proof")
        || compact.contains("productionproof")
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
    if tokens.iter().any(|token| token == "fitting") {
        return Some("fitting");
    }
    for &(first, second, compact_name, label) in GOAL_WORK_PAIRS {
        if compact.contains(compact_name) || adjacent_tokens(&tokens, first, second) {
            return Some(label);
        }
    }
    if let Some(label) = repo_entrypoint_context_label(&tokens) {
        return Some(label);
    }
    if adjacent_numbered_label(&tokens, "gate") {
        return Some("gate_number");
    }
    if adjacent_numbered_label(&tokens, "phase") {
        return Some("phase_number");
    }
    if let Some(label) = standalone_goal_work_token_label(&tokens) {
        return Some(label);
    }
    None
}

fn session_history_status_label(lower: &str) -> Option<&'static str> {
    match lower.trim() {
        "worker thread:" => Some("session_history_worker_thread"),
        "thread id:" => Some("session_history_thread_id"),
        "current phase:" => Some("session_history_current_phase"),
        "phase progress:" => Some("session_history_phase_progress"),
        "backlog item:" => Some("session_history_backlog_item"),
        "receipt status:" => Some("session_history_receipt_status"),
        "completion claim:" => Some("session_history_completion_claim"),
        _ => None,
    }
}
fn authority_like_string(text: &str) -> bool {
    let trimmed = text.trim();
    if telemetry_instance_literal(trimmed) {
        return false;
    }
    !trimmed.is_empty()
        && (trimmed.contains('/')
            || trimmed.ends_with(".json")
            || trimmed.ends_with(".jsonl")
            || trimmed.ends_with(".rs")
            || trimmed.ends_with(".md")
            || (trimmed.contains(['_', '-', '.'])
                && trimmed
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))))
}

fn telemetry_instance_literal(text: &str) -> bool {
    text.starts_with("run-") || text.starts_with("corr-") || text.starts_with("sha256:")
}

fn standalone_goal_work_token_label(tokens: &[String]) -> Option<&'static str> {
    for token in tokens {
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
    }
    None
}

pub(crate) fn generic_source_leaf_label(stem: &str) -> Option<&'static str> {
    generic_bucket_label(stem, true)
}

fn generic_bucket_label(value: &str, source_leaf: bool) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    let tokens = semantic_tokens(value);
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
    for &(exact, label) in &[("common", "common"), ("shared", "shared"), ("misc", "misc")] {
        if lower == exact || (exact == "common" && tokens.iter().any(|token| token == exact)) {
            return Some(label);
        }
    }
    if source_leaf && lower == "nodes" {
        return Some("nodes");
    }
    None
}

fn adjacent_tokens(tokens: &[String], first: &str, second: &str) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0] == first && window[1] == second)
}

fn adjacent_numbered_label(tokens: &[String], label: &str) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0] == label && window[1].chars().all(|ch| ch.is_ascii_digit()))
}
