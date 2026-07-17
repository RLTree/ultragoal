use serde_json::{Value, json};
use std::path::{Path, PathBuf};

mod receipt;

#[cfg(test)]
const DEFAULT_RECEIPT: &str = "validation_artifacts/openai/call-receipt.json";
#[cfg(test)]
const DEFAULT_MODE: &str = "no_network";
#[cfg(test)]
const DEFAULT_MODEL: &str = "none:no-network";
#[cfg(test)]
const DEFAULT_ENDPOINT: &str = "none:no-network";
#[cfg(test)]
const DEFAULT_PURPOSE: &str = "openai-boundary-proof";
#[cfg(test)]
const DEFAULT_SCHEMA_ID: &str = "harness-ultragoal.openai-call-receipt.v1";

#[derive(Debug)]
pub(crate) struct CallCommand {
    pub(super) receipt: PathBuf,
    pub(super) provider_mode: String,
    pub(super) model_identity: String,
    pub(super) endpoint_api_family: String,
    pub(super) purpose: String,
    pub(super) schema_id: String,
    pub(super) input_digest: String,
    pub(super) output_digest: String,
    pub(super) provider_policy: PathBuf,
    pub(super) budget_class: String,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<CallCommand, String> {
    let provider_mode =
        super::opt_string(raw, "--mode").unwrap_or_else(|| DEFAULT_MODE.to_string());
    if !["no_network", "offline_fixture", "local_mock", "openai_live"]
        .contains(&provider_mode.as_str())
    {
        return Err("openai call prove supports no_network, offline_fixture, local_mock, or openai_live only".into());
    }
    let budget_class = super::opt_string(raw, "--budget-class")
        .unwrap_or_else(|| super::budget::default_class(&provider_mode).to_string());
    Ok(CallCommand {
        receipt: super::opt_path(raw, "--receipt")
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT)),
        model_identity: super::opt_string(raw, "--model")
            .unwrap_or_else(|| default_model(&provider_mode).to_string()),
        endpoint_api_family: super::opt_string(raw, "--endpoint")
            .unwrap_or_else(|| default_endpoint(&provider_mode).to_string()),
        purpose: super::opt_string(raw, "--purpose").unwrap_or_else(|| DEFAULT_PURPOSE.to_string()),
        schema_id: super::opt_string(raw, "--schema-id")
            .unwrap_or_else(|| DEFAULT_SCHEMA_ID.to_string()),
        input_digest: super::opt_string(raw, "--input-digest")
            .unwrap_or_else(|| crate::digest::ZERO.to_string()),
        output_digest: super::opt_string(raw, "--output-digest")
            .unwrap_or_else(|| crate::digest::ZERO.to_string()),
        provider_policy: super::opt_path(raw, "--provider-policy")
            .unwrap_or_else(|| PathBuf::from(super::budget::DEFAULT_POLICY)),
        provider_mode,
        budget_class,
    })
}

pub(crate) fn run(root: &Path, command: &CallCommand) -> Result<i32, String> {
    let receipt = build_call_receipt(root, command)?;
    let path =
        crate::output_path::claim_artifact_path(root, &command.receipt, "OpenAI call receipt")?;
    crate::json_boundary::write_json(&path, &receipt)?;
    receipt::print_receipt(&command.receipt, &receipt);
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
    let live = receipt::live_observation(root, command, &budget)?;
    let prompt_input_digest = receipt::prompt_input_digest(command, live.as_ref());
    let output_digest = receipt::output_digest(command, live.as_ref());
    let mut failures = receipt::receipt_failures(&prompt_input_digest, &output_digest, &budget);
    if let Some(observation) = &live {
        failures.extend(observation.failures.clone());
    }
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
            runtime: None,
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
        "prompt_input_digest": prompt_input_digest,
        "output_digest": output_digest,
        "schema_id": command.schema_id,
        "request_id": receipt::request_id(command, live.as_ref()),
        "token_cost_rate_limit": receipt::token_cost_rate_limit(
            &candidate,
            command,
            &budget,
            live.as_ref()
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
        return Ok(receipt::secret_leak_receipt(value));
    }
    Ok(value)
}

#[cfg(test)]
fn default_model(provider_mode: &str) -> &'static str {
    if provider_mode == "openai_live" {
        "gpt-5.4-nano"
    } else {
        DEFAULT_MODEL
    }
}

#[cfg(test)]
fn default_endpoint(provider_mode: &str) -> &'static str {
    if provider_mode == "openai_live" {
        "responses"
    } else {
        DEFAULT_ENDPOINT
    }
}

#[cfg(test)]
mod tests;
