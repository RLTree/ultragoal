pub(super) const GOAL_WORK_PAIRS: &[(&str, &str, &str, &str)] = &[
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

pub(super) fn source_name_violation_label(
    tokens: &[String],
    compact: &str,
) -> Option<&'static str> {
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
        (
            "observability",
            "gate",
            "observabilitygate",
            "observability_product_closure",
        ),
        ("validation", "proof", "validationproof", "validation_proof"),
    ] {
        if compact == compact_name || adjacent_tokens(tokens, first, second) {
            return Some(label);
        }
    }
    None
}

pub(super) fn generic_bucket_label(value: &str, source_leaf: bool) -> Option<&'static str> {
    let lower = value.to_ascii_lowercase();
    let tokens = super::semantic_tokens::semantic_tokens(value);
    let generic_label = |token: &str| match token {
        "support" => Some("support"),
        "helper" | "helpers" => Some("helper"),
        "utils" | "utility" | "utilities" => Some("utils"),
        "common" => Some("common"),
        "shared" => Some("shared"),
        "misc" => Some("misc"),
        _ => None,
    };
    if tokens.iter().all(|token| generic_label(token).is_some()) {
        return tokens.first().and_then(|token| generic_label(token));
    }
    if source_leaf && lower == "nodes" {
        return Some("nodes");
    }
    None
}

pub(super) fn adjacent_tokens(tokens: &[String], first: &str, second: &str) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0] == first && window[1] == second)
}

pub(super) fn adjacent_numbered_label(tokens: &[String], label: &str) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0] == label && window[1].chars().all(|ch| ch.is_ascii_digit()))
}
