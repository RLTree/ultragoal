use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const DEFAULT_RECEIPT: &str = "validation_artifacts/openai/call-receipt.json";
const DEFAULT_MODE: &str = "no_network";
const DEFAULT_MODEL: &str = "none:no-network";
const DEFAULT_ENDPOINT: &str = "none:no-network";
const DEFAULT_PURPOSE: &str = "openai-boundary-proof";
const DEFAULT_SCHEMA_ID: &str = "harness-ultragoal.openai-call-receipt.v1";

#[derive(Debug)]
pub(crate) struct CallCommand {
    receipt: PathBuf,
    provider_mode: String,
    model_identity: String,
    endpoint_api_family: String,
    purpose: String,
    schema_id: String,
    input_digest: String,
    output_digest: String,
    provider_policy: PathBuf,
    budget_class: String,
}

pub(crate) fn parse(raw: &[String]) -> Result<CallCommand, String> {
    let provider_mode =
        super::opt_string(raw, "--mode").unwrap_or_else(|| DEFAULT_MODE.to_string());
    if !["no_network", "offline_fixture", "local_mock"].contains(&provider_mode.as_str()) {
        return Err(
            "openai call prove supports no_network, offline_fixture, or local_mock only".into(),
        );
    }
    let budget_class = super::opt_string(raw, "--budget-class")
        .unwrap_or_else(|| super::budget::default_class(&provider_mode).to_string());
    Ok(CallCommand {
        receipt: super::opt_path(raw, "--receipt")
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT)),
        provider_mode,
        model_identity: super::opt_string(raw, "--model")
            .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        endpoint_api_family: super::opt_string(raw, "--endpoint")
            .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
        purpose: super::opt_string(raw, "--purpose").unwrap_or_else(|| DEFAULT_PURPOSE.to_string()),
        schema_id: super::opt_string(raw, "--schema-id")
            .unwrap_or_else(|| DEFAULT_SCHEMA_ID.to_string()),
        input_digest: super::opt_string(raw, "--input-digest")
            .unwrap_or_else(|| crate::digest::ZERO.to_string()),
        output_digest: super::opt_string(raw, "--output-digest")
            .unwrap_or_else(|| crate::digest::ZERO.to_string()),
        provider_policy: super::opt_path(raw, "--provider-policy")
            .unwrap_or_else(|| PathBuf::from(super::budget::DEFAULT_POLICY)),
        budget_class,
    })
}

pub(crate) fn run(root: &Path, command: &CallCommand) -> Result<i32, String> {
    let receipt = build_call_receipt(root, command)?;
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

pub(crate) fn build_call_receipt(root: &Path, command: &CallCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let budget = super::budget::load(
        root,
        &command.provider_policy,
        &command.budget_class,
        &command.provider_mode,
    );
    let failures = receipt_failures(command, &budget);
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
            subcommand: "call prove",
            operation: "openai.call.prove",
            surface: "source",
            law_id: super::LAW_ID,
            check_id: super::LAW_ID,
            claim_id: "openai_call_receipt_boundary",
            artifact_path: "schemas/openai-call-receipt.schema.json",
            receipt_path: &receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "schema_invalid"
            },
            why_failed: &failure_text,
            where_failed: "openai call prove arguments",
            next_repair: if status == "pass" {
                "none"
            } else {
                "provide sha256 digests and a bounded OpenAI provider budget policy"
            },
            claim_impact: if status == "pass" {
                "supports_openai_call_receipt_shape_only"
            } else {
                "blocks_openai_call_claims"
            },
            blocked_claims: super::config::blocked_claims(),
            supported_claims: if status == "pass" {
                vec!["openai_call_receipt_boundary".to_string()]
            } else {
                Vec::new()
            },
            emit: true,
        },
    )?;
    let value = json!({
        "schema": "harness-ultragoal.openai-call-receipt.v1",
        "status": status,
        "candidate_digest": candidate,
        "provider_mode": command.provider_mode,
        "provider_policy_path": command.provider_policy.to_string_lossy().replace('\\', "/"),
        "provider_policy_digest": budget.policy_digest,
        "budget_class": budget.budget_class,
        "model_identity": command.model_identity,
        "endpoint_api_family": command.endpoint_api_family,
        "purpose": command.purpose,
        "prompt_input_digest": command.input_digest,
        "output_digest": command.output_digest,
        "schema_id": command.schema_id,
        "request_id": request_id(&command.provider_mode),
        "token_cost_rate_limit": super::budget::cost_rate_limit(
            &candidate,
            &command.provider_mode,
            &budget
        ),
        "timeout_retry_backoff": super::budget::timeout_retry_backoff(&budget),
        "cache_policy": super::budget::cache_policy(&budget),
        "run_id": observability.get("run_id").and_then(Value::as_str).unwrap_or("missing"),
        "correlation_id": observability
            .get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("missing"),
        "redaction_status": "pass",
        "model_output_authority": "observation_only_until_cli_schema_validated",
        "observability_receipt": observability,
        "supported_claims": if status == "pass" {
            vec!["openai_call_receipt_boundary"]
        } else {
            Vec::<&str>::new()
        },
        "blocked_claims": super::config::blocked_claims(),
        "claim_impact": if status == "pass" {
            "supports_openai_call_receipt_shape_only_not_model_authority"
        } else {
            "withheld_or_blocked"
        },
        "failures": failures
    });
    if super::policy::contains_secret_shape(&value) {
        return Ok(secret_leak_receipt(value));
    }
    Ok(value)
}

fn receipt_failures(command: &CallCommand, budget: &super::budget::BudgetSelection) -> Vec<String> {
    let mut failures = Vec::new();
    if !is_sha256(&command.input_digest) {
        failures.push("openai_call_prompt_input_digest_invalid".to_string());
    }
    if !is_sha256(&command.output_digest) {
        failures.push("openai_call_output_digest_invalid".to_string());
    }
    failures.extend(budget.failures.clone());
    failures
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|tail| tail.len() == 64 && tail.chars().all(|ch| ch.is_ascii_hexdigit()))
}

fn request_id(mode: &str) -> String {
    match mode {
        "no_network" => "not_available:no_network".to_string(),
        "offline_fixture" => "not_available:offline_fixture".to_string(),
        _ => "not_available:local_mock".to_string(),
    }
}

fn secret_leak_receipt(mut value: Value) -> Value {
    value["status"] = json!("fail");
    value["redaction_status"] = json!("fail");
    value["supported_claims"] = json!([]);
    value["claim_impact"] = json!("withheld_or_blocked");
    value["failures"] = json!(["openai_call_receipt_secret_shape_detected"]);
    value
}

fn print_receipt(receipt: &Path, value: &Value) {
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("fail");
    println!(
        "ultragoal-openai-call {status} candidate={} receipt={} run_id={} correlation_id={} claim_impact={} supported_claims={} unsupported_claims={}",
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        receipt.display(),
        value
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("correlation_id")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        super::csv(value.get("supported_claims")),
        super::csv(value.get("blocked_claims"))
    );
}
