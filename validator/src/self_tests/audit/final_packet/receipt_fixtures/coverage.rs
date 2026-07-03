use serde_json::{Value, json};
use std::path::Path;

pub(super) fn ref_for(root: &Path, current: &str) -> Value {
    super::ref_for(
        root,
        "validation_artifacts/coverage/coverage-receipt.json",
        &json!({
            "schema":"harness-ultragoal.coverage-receipt.v1",
            "claim_id":"CLAIM-100",
            "command":"ultragoal coverage prove",
            "tool":"cargo-llvm-cov",
            "source_tree_digest":crate::self_tests::boundaries::workspace_fixtures::sha('2'),
            "coverage_manifest_digest":crate::self_tests::boundaries::workspace_fixtures::sha('3'),
            "coverage_command_digest":crate::self_tests::boundaries::workspace_fixtures::sha('4'),
            "changed_files_digest":crate::self_tests::boundaries::workspace_fixtures::sha('5'),
            "tool_version":"test",
            "workspace_root":".",
            "target_revision":{"kind":"package_digest","value":current},
            "command_started_at":"2026-06-27T00:00:00Z",
            "command_completed_at":"2026-06-27T00:00:01Z",
            "command_exit":0,
            "machine_readable_report":{"path":"validation_artifacts/coverage/report.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('6')},
            "generated_by":"coverage-command",
            "percent_source":"machine_readable_report",
            "target_paths":["validator/src"],
            "measured_dimensions":["line"],
            "coverage":{"percent":100.0,"floor_percent":100.0,"policy":"100_percent_required"},
            "uncovered_records":[],
            "exclusions":[],
            "generated_at":"2026-06-27T00:00:01Z",
            "claim_ceiling":"supports_complete_coverage_claim",
            "supported_claim_classes":["complete_coverage"],
            "blocked_claim_classes":[
                "completion",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "final_packet_correctness",
                "update_goal_eligibility",
                "app_registry_or_reviewer_exposure"
            ]
        }),
    )
}
