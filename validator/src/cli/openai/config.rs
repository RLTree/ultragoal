use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_POLICY: &str = "docs/openai-key-policy.json";
pub(crate) const DEFAULT_RECEIPT: &str = "validation_artifacts/openai/config-receipt.json";

#[derive(Debug)]
pub(crate) struct ConfigCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) policy: PathBuf,
}

pub(crate) fn run(root: &Path, command: &ConfigCommand) -> Result<i32, String> {
    let receipt = build_config_receipt(root, command)?;
    let path = if command.receipt.is_absolute() {
        command.receipt.clone()
    } else {
        root.join(&command.receipt)
    };
    crate::json_boundary::write_json(&path, &receipt)?;
    print_receipt(&command.receipt, &receipt);
    Ok(i32::from(
        receipt.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn build_config_receipt(root: &Path, command: &ConfigCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let policy = super::policy::load(root, &command.policy);
    let failures = policy.failures();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let failure_text = if failures.is_empty() {
        "none".to_string()
    } else {
        failures.join("; ")
    };
    let receipt_rel = command.receipt.to_string_lossy().replace('\\', "/");
    let observability = crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal openai",
            subcommand: "config prove",
            operation: "openai.config.prove",
            surface: "source",
            law_id: super::LAW_ID,
            check_id: super::LAW_ID,
            claim_id: "openai_config_redacted_resolution",
            artifact_path: DEFAULT_POLICY,
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "schema_invalid"
            },
            why_failed: &failure_text,
            where_failed: DEFAULT_POLICY,
            next_repair: if status == "pass" {
                "none"
            } else {
                "repair OpenAI key policy or local untracked env destination"
            },
            claim_impact: if status == "pass" {
                "supports_openai_config_resolution_only"
            } else {
                "blocks_openai_dependent_claims"
            },
            blocked_claims: blocked_claims(),
            supported_claims: if status == "pass" {
                vec!["openai_config_redacted_resolution".to_string()]
            } else {
                Vec::new()
            },
            emit: true,
        },
    )?;
    let value = json!({
        "schema": super::RECEIPT_SCHEMA,
        "schema_version": "v1",
        "issuer": {
            "tool": "ultragoal",
            "authority": "cli_control_plane"
        },
        "generated_at": crate::audit::clock::now_iso(),
        "status": status,
        "candidate_digest": candidate,
        "policy_path": command.policy.to_string_lossy().replace('\\', "/"),
        "env_var": "OPENAI_API_KEY",
        "key_resolution": policy.resolution_value(),
        "provider_modes": {
            "live": "requires_same_candidate_call_receipt",
            "offline_fixture": "deterministic_tests_only",
            "local_mock": "non_authoritative_observation",
            "no_network": "deterministic_no_provider"
        },
        "model_output_authority": "observation_only_until_cli_schema_validated",
        "redaction_status": if policy.secret_leak_free() { "pass" } else { "fail" },
        "secret_material_serialized": false,
        "observability_receipt": observability,
        "claim_ceiling": if status == "pass" {
            "openai_config_resolution_only"
        } else {
            "withheld_or_blocked"
        },
        "supported_claims": if status == "pass" {
            vec!["openai_config_redacted_resolution"]
        } else {
            Vec::<&str>::new()
        },
        "blocked_claims": blocked_claims(),
        "failures": failures
    });
    if super::policy::contains_secret_shape(&value) {
        return Ok(secret_leak_receipt(value));
    }
    Ok(value)
}

fn secret_leak_receipt(mut value: Value) -> Value {
    value["status"] = json!("fail");
    value["claim_ceiling"] = json!("withheld_or_blocked");
    value["redaction_status"] = json!("fail");
    value["supported_claims"] = json!([]);
    value["failures"] = json!(["openai_config_receipt_secret_shape_detected"]);
    value
}

fn print_receipt(receipt: &Path, value: &Value) {
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail");
    let obs = value.get("observability_receipt").unwrap_or(&Value::Null);
    println!(
        "ultragoal-openai-config {status} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        receipt.display(),
        obs.get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        obs.get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        obs.get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        super::csv(value.get("supported_claims")),
        super::csv(value.get("blocked_claims"))
    );
    if status != "pass" {
        println!(
            "failed_check={} why={} where={} claim_impact=blocks_openai_dependent_claims next_repair=repair_openai_key_policy_or_local_env receipt={}",
            super::LAW_ID,
            super::csv(value.get("failures")),
            value
                .get("policy_path")
                .and_then(Value::as_str)
                .unwrap_or(DEFAULT_POLICY),
            receipt.display()
        );
    }
}

pub(crate) fn blocked_claims() -> Vec<String> {
    [
        "completion",
        "readiness",
        "release",
        "reviewer_exposure",
        "app_registry_exposure",
        "final_packet_correctness",
        "update_goal_eligibility",
        "product_success",
        "model_output_authority",
        "live_model_claim",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}
