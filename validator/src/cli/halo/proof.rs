use super::HaloCommand;
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn build_receipt(root: &Path, command: &HaloCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let registry =
        crate::json_boundary::read_json(&root.join(super::REGISTRY)).unwrap_or(Value::Null);
    let failures = registry_failures(&registry);
    let app = super::detect::app(&command.app_path);
    let cli_available = super::detect::command_on_path("halo");
    let mode = if app.observed {
        "desktop_manual"
    } else {
        "unavailable"
    };
    let authority = if app.observed {
        "manual_observation_only"
    } else {
        "fail_closed"
    };
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let failure_text = if failures.is_empty() {
        "none".to_string()
    } else {
        failures.join("; ")
    };
    let receipt_rel = command.receipt.to_string_lossy().replace('\\', "/");
    let observability = observability(root, status, &failure_text, &receipt_rel)?;
    let mut value = json!({
        "schema": super::SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "adapter_registry_path": super::REGISTRY,
        "adapter_registry_digest": digest_or_zero(root, super::REGISTRY),
        "invocation_mode": mode,
        "bundle_observed": app.observed,
        "bundle_path_class": app.path_class,
        "bundle_identifier": app.identifier,
        "bundle_version": app.version,
        "cli_available": cli_available,
        "api_available": false,
        "authority_class": authority,
        "privacy_boundary": "category_only_inputs_no_raw_private_logs_or_secrets",
        "raw_halo_authority": "observation_only_until_cli_adapter_ranking_and_validation",
        "observability_receipt": observability,
        "supported_claims": if app.observed {
            vec!["halo_desktop_manual_capability_observed"]
        } else {
            Vec::<&str>::new()
        },
        "blocked_claims": blocked_claims(),
        "claim_impact": "supports_halo_capability_detection_only_blocks_ranking_readiness_update_goal",
        "failures": failures
    });
    if crate::cli::openai::policy::contains_secret_shape(&value) {
        value["status"] = json!("fail");
        value["supported_claims"] = json!([]);
        value["claim_impact"] = json!("withheld_or_blocked");
        value["failures"] = json!(["halo_capability_receipt_secret_shape_detected"]);
    }
    Ok(value)
}

pub(super) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    require_str(receipt, "schema", super::SCHEMA, &mut out);
    require_str(receipt, "status", "pass", &mut out);
    require_str(receipt, "candidate_digest", &candidate, &mut out);
    require_str(
        receipt,
        "raw_halo_authority",
        "observation_only_until_cli_adapter_ranking_and_validation",
        &mut out,
    );
    require_digest(
        root,
        receipt,
        "adapter_registry_digest",
        super::REGISTRY,
        &mut out,
    );
    if !blocks_required_claims(receipt) {
        out.push("halo_capability_receipt_missing_claim_blockers".to_string());
    }
    if receipt.get("authority_class").and_then(Value::as_str) != Some("manual_observation_only")
        && receipt.get("authority_class").and_then(Value::as_str) != Some("fail_closed")
    {
        out.push("halo_capability_authority_overbroad".to_string());
    }
    check_observability(receipt, &candidate, &mut out);
    out
}

fn observability(root: &Path, status: &str, why: &str, receipt: &str) -> Result<Value, String> {
    crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal halo",
            subcommand: "capability prove",
            operation: "halo.capability.prove",
            surface: "source",
            law_id: super::LAW_ID,
            check_id: super::LAW_ID,
            claim_id: "halo_capability_boundary",
            artifact_path: "schemas/halo-capability-receipt.schema.json",
            receipt_path: receipt,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "schema_invalid"
            },
            why_failed: why,
            where_failed: "HALO app/adapter capability detection",
            next_repair: if status == "pass" {
                "none"
            } else {
                "repair HALO adapter registry or capability receipt schema"
            },
            claim_impact: "supports_halo_capability_detection_only_blocks_ranking_readiness_update_goal",
            blocked_claims: blocked_claims().into_iter().map(str::to_string).collect(),
            supported_claims: if status == "pass" {
                vec!["halo_capability_boundary".to_string()]
            } else {
                Vec::new()
            },
            runtime: None,
            emit: true,
        },
    )
}

fn registry_failures(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.halo-adapter-registry.v1")
    {
        out.push("halo_adapter_registry_wrong_schema".to_string());
    }
    if value.get("law_id").and_then(Value::as_str) != Some(super::LAW_ID) {
        out.push("halo_adapter_registry_wrong_law_id".to_string());
    }
    for mode in ["desktop_manual", "cli", "api", "fixture", "unavailable"] {
        if !mode_ids(value).iter().any(|item| item == mode) {
            out.push(format!("halo_adapter_registry_mode_missing:{mode}"));
        }
    }
    for forbidden in [
        "halo_manual_output_as_deterministic_authority",
        "halo_desktop_presence_as_ranked_change_proof",
        "halo_recommendation_as_readiness",
        "halo_recommendation_as_update_goal",
        "halo_ranking_without_codex_handoff",
        "halo_ranking_without_validation_closure",
    ] {
        if !array_strings(value.pointer("/forbidden_substitutions")).contains(&forbidden) {
            out.push(format!("halo_forbidden_substitution_missing:{forbidden}"));
        }
    }
    out
}

fn mode_ids(value: &Value) -> Vec<String> {
    value
        .pointer("/modes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn array_strings(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string())
}

fn require_str(receipt: &Value, key: &str, expected: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(expected) {
        out.push(format!("halo_capability_receipt_field_mismatch:{key}"));
    }
}

fn require_digest(root: &Path, receipt: &Value, key: &str, rel: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(digest_or_zero(root, rel).as_str()) {
        out.push(format!("halo_capability_receipt_digest_mismatch:{key}"));
    }
}

fn blocks_required_claims(receipt: &Value) -> bool {
    let claims = receipt
        .get("blocked_claims")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    blocked_claims().iter().all(|claim| claims.contains(claim))
}

fn check_observability(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    let obs = receipt.get("observability_receipt").unwrap_or(&Value::Null);
    if obs.get("schema").and_then(Value::as_str)
        != Some(crate::cli::observe::command::RECEIPT_SCHEMA)
        || obs.get("status").and_then(Value::as_str) != Some("pass")
        || obs.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || obs.get("law_id").and_then(Value::as_str) != Some(super::LAW_ID)
    {
        out.push("halo_capability_observability_binding_invalid".to_string());
    }
}

fn blocked_claims() -> Vec<&'static str> {
    vec![
        "completion",
        "readiness",
        "release",
        "final_packet_correctness",
        "update_goal_eligibility",
        "product_success",
        "registry_exposure",
        "reviewer_exposure",
        "halo_ranked_change_authority",
        "halo_recommendation_readiness",
        "improvement_loop_closure",
    ]
}
