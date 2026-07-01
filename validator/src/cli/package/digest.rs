use std::path::Path;

const RECEIPT_REL: &str = "validation_artifacts/observability/package-digest.json";

pub(crate) fn run(root: &Path) -> Result<i32, String> {
    let value = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal package",
            subcommand: "digest",
            operation: "package.digest",
            surface: "source_package",
            law_id: crate::cli::observe::types::LAW_ID,
            check_id: "package-digest-observability-binding",
            claim_id: "source_package_digest",
            artifact_path: "plugin-manifest-draft.json",
            receipt_path: RECEIPT_REL,
            status: "pass",
            failure_class: "none",
            why_failed: "none",
            where_failed: "none",
            next_repair: "none",
            claim_impact: "supports_source_package_digest_only",
            blocked_claims: blocked_claims(),
            supported_claims: vec!["source_package_digest".to_string()],
            runtime: None,
            emit: true,
        },
    )?;
    crate::json_boundary::write_json(&root.join(RECEIPT_REL), &value)?;
    print_receipt(&value);
    Ok(0)
}

fn print_receipt(value: &serde_json::Value) {
    let digest = value["candidate_digest"].as_str().unwrap_or("<missing>");
    println!("{digest}");
    println!(
        "ultragoal-package-digest pass proven=source_package_digest candidate={digest} receipt={RECEIPT_REL} run_id={} correlation_id={} claim_impact={} supported_claims=source_package_digest unsupported_claims={}",
        value["run_id"].as_str().unwrap_or("<missing>"),
        value["correlation_id"].as_str().unwrap_or("<missing>"),
        value["claim_impact"].as_str().unwrap_or("<missing>"),
        blocked_claims().join(",")
    );
}

fn blocked_claims() -> Vec<String> {
    [
        "completion",
        "readiness",
        "release",
        "reviewer_exposure",
        "app_registry_exposure",
        "final_packet_correctness",
        "update_goal_eligibility",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}
