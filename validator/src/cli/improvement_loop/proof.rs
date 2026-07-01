use super::{ImprovementLoopCommand, registry};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn build_receipt(
    root: &Path,
    command: &ImprovementLoopCommand,
) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let mut failures = Vec::new();
    let registry_doc = read_json(root, super::REGISTRY, &mut failures);
    failures.extend(registry::registry_failures(root, &registry_doc));
    let promptfoo_digest =
        digest_or_zero(root, "validation_artifacts/promptfoo/adapter-receipt.json");
    let halo_digest = digest_or_zero(root, "validation_artifacts/halo/capability-receipt.json");
    if !registry::complete_same_candidate(&registry_doc) {
        failures.push("improvement_loop_closure_missing_or_incomplete".to_string());
    }
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let failure_text = if failures.is_empty() {
        "none".to_string()
    } else {
        failures.join("; ")
    };
    let receipt_rel = command.receipt.to_string_lossy().replace('\\', "/");
    let observability = observability(root, status, &failure_text, &receipt_rel)?;
    Ok(json!({
        "schema": super::RECEIPT_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "registry_path": super::REGISTRY,
        "registry_digest": digest_or_zero(root, super::REGISTRY),
        "loop_ids": registry::loop_ids(&registry_doc),
        "promptfoo_adapter_receipt_digest": promptfoo_digest,
        "halo_capability_receipt_digest": halo_digest,
        "raw_observation_authority": "observation_only_until_cli_parsed_loop_receipt",
        "observability_receipt": observability,
        "supported_claims": if status == "pass" {
            vec!["harness_improvement_loop_closure"]
        } else {
            Vec::<&str>::new()
        },
        "blocked_claims": blocked_claims(),
        "claim_impact": if status == "pass" {
            "supports_same_candidate_improvement_loop_closure_only"
        } else {
            "withheld_or_blocked"
        },
        "failures": failures
    }))
}

pub(super) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    registry::require_str(receipt, "schema", super::RECEIPT_SCHEMA, &mut out);
    registry::require_str(receipt, "status", "pass", &mut out);
    registry::require_str(receipt, "candidate_digest", &candidate, &mut out);
    registry::require_str(
        receipt,
        "raw_observation_authority",
        "observation_only_until_cli_parsed_loop_receipt",
        &mut out,
    );
    require_digest(root, receipt, "registry_digest", super::REGISTRY, &mut out);
    if !blocks_required_claims(receipt) {
        out.push("improvement_loop_receipt_missing_claim_blockers".to_string());
    }
    check_observability(receipt, &candidate, &mut out);
    out
}

fn observability(root: &Path, status: &str, why: &str, receipt: &str) -> Result<Value, String> {
    crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal improvement-loop",
            subcommand: "prove",
            operation: "improvement_loop.prove",
            surface: "source",
            law_id: super::LAW_ID,
            check_id: super::LAW_ID,
            claim_id: "harness_improvement_loop_closure",
            artifact_path: "schemas/improvement-loop-receipt.schema.json",
            receipt_path: receipt,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "claim_blocked"
            },
            why_failed: why,
            where_failed: "improvement loop registry and closure receipt",
            next_repair: if status == "pass" {
                "none"
            } else {
                "complete trace-feedback-eval-ranking-handoff-validation-promotion loop with same-candidate evidence"
            },
            claim_impact: if status == "pass" {
                "supports_harness_improvement_loop_closure_only"
            } else {
                "blocks_self_improving_learning_regression_prevention_update_goal_claims"
            },
            blocked_claims: blocked_claim_strings(),
            supported_claims: if status == "pass" {
                vec!["harness_improvement_loop_closure".to_string()]
            } else {
                Vec::new()
            },
            runtime: None,
            emit: true,
        },
    )
}

fn read_json(root: &Path, rel: &str, failures: &mut Vec<String>) -> Value {
    crate::json_boundary::read_json(&root.join(rel)).unwrap_or_else(|err| {
        failures.push(format!(
            "improvement_loop_json_missing_or_malformed:{rel}:{err}"
        ));
        Value::Null
    })
}

fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string())
}

fn require_digest(root: &Path, receipt: &Value, key: &str, rel: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(digest_or_zero(root, rel).as_str()) {
        out.push(format!("improvement_loop_receipt_digest_mismatch:{key}"));
    }
}

fn check_observability(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    let obs = receipt.get("observability_receipt").unwrap_or(&Value::Null);
    if obs.get("schema").and_then(Value::as_str) != Some(crate::cli::observe::types::RECEIPT_SCHEMA)
        || obs.get("status").and_then(Value::as_str) != Some("pass")
        || obs.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || obs.get("law_id").and_then(Value::as_str) != Some(super::LAW_ID)
        || obs.get("check_id").and_then(Value::as_str) != Some(super::LAW_ID)
    {
        out.push("improvement_loop_receipt_observability_binding_invalid".to_string());
    }
}

fn blocks_required_claims(receipt: &Value) -> bool {
    let claims = registry::array_strings(receipt.get("blocked_claims"));
    blocked_claims().iter().all(|claim| claims.contains(claim))
}

fn blocked_claims() -> Vec<&'static str> {
    vec![
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "update_goal_eligibility",
        "self_improving_claim",
        "learning_claim",
        "regression_prevention_claim",
        "product_learning_claim",
        "reviewer_exposure",
        "app_registry_exposure",
    ]
}

fn blocked_claim_strings() -> Vec<String> {
    blocked_claims().into_iter().map(str::to_string).collect()
}
