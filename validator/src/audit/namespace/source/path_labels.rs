use super::{
    label_patterns::{
        GOAL_WORK_PAIRS, adjacent_numbered_label, adjacent_tokens, generic_bucket_label,
        repo_entrypoint_context_label, source_name_violation_label,
    },
    semantic_tokens::semantic_tokens,
};

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
    if lower.contains("parent-session")
        || lower.contains("parent_session")
        || compact.contains("parentsession")
    {
        return Some("session_history_builder_contract");
    }
    if let Some(label) = session_history_status_label(&lower) {
        return Some(label);
    }
    if adjacent_numbered_label(&tokens, "gate") {
        return Some("gate_number");
    }
    if adjacent_tokens(&tokens, "observability", "gate") {
        return Some("observability_product_closure");
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
