const DETECTOR_CATALOG_PATHS: &[&str] = &[
    "validator/src/audit/namespace/source/path_labels.rs",
    "validator/src/audit/namespace/source/label_patterns.rs",
    "validator/src/audit/namespace/source/identifiers.rs",
    "validator/src/audit/namespace/source/rejection_ownership.rs",
    "validator/src/audit/namespace/source/string_labels.rs",
];

const DETECTOR_REJECTION_LABELS: &[&str] = &[
    "checkpoint",
    "checkpoint_progress",
    "evidence_posture",
    "evidence_status",
    "fit",
    "fit_command",
    "fit_goal",
    "fit_path",
    "fit_slice",
    "fitting",
    "gate_number",
    "observability_product_closure",
    "phase",
    "phase4_rebind",
    "phase_number",
    "production_evidence",
    "production_proof",
    "proof_state",
    "proof_status",
    "root_phase",
    "session_history_backlog_item",
    "session_history_builder_contract",
    "session_history_completion_claim",
    "session_history_current_phase",
    "session_history_phase_progress",
    "session_history_receipt_status",
    "session_history_thread_id",
    "session_history_worker_thread",
    "slice",
    "todo",
    "todo_repair",
    "validation_proof",
    "workstream",
    "wip",
];

const NEGATIVE_FIXTURE_PATHS: &[&str] = &[
    "validator/src/self_tests/namespace/binding/semantic_names.rs",
    "validator/src/self_tests/namespace/binding/path_label_edges.rs",
    "validator/src/self_tests/namespace/binding/raw_string_edges.rs",
    "validator/src/self_tests/namespace/binding/law.rs",
    "validator/src/self_tests/namespace/red_fixtures.rs",
];

const NEGATIVE_FIXTURE_LABELS: &[&str] = &[
    "common",
    "checkpoint",
    "checkpoint_progress",
    "evidence_posture",
    "evidence_status",
    "fit_command",
    "fit_goal",
    "fit_path",
    "fit_slice",
    "helper",
    "support",
    "fitting",
    "gate_number",
    "phase",
    "phase4_rebind",
    "phase_number",
    "production_evidence",
    "production_proof",
    "progress",
    "proof_state",
    "proof_status",
    "root_phase",
    "session_history_backlog_item",
    "session_history_completion_claim",
    "session_history_current_phase",
    "session_history_phase_progress",
    "session_history_receipt_status",
    "session_history_thread_id",
    "session_history_worker_thread",
    "slice",
    "todo_repair",
];

pub(super) fn catalog_or_fixture_owns_label(rel: &str, label: &str) -> bool {
    detector_catalog_owns_label(rel, label) || negative_fixture_owns_label(rel, label)
}

fn detector_catalog_owns_label(rel: &str, label: &str) -> bool {
    DETECTOR_CATALOG_PATHS.contains(&rel) && DETECTOR_REJECTION_LABELS.contains(&label)
}

fn negative_fixture_owns_label(rel: &str, label: &str) -> bool {
    NEGATIVE_FIXTURE_PATHS.contains(&rel) && NEGATIVE_FIXTURE_LABELS.contains(&label)
}
