use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const DEFAULT_RECEIPT: &str = "validation_artifacts/openai/model-output-authority.json";
const DEFAULT_CALL_RECEIPT: &str = "validation_artifacts/openai/call-receipt.json";
const DEFAULT_PARSER_SCHEMA: &str = "harness-ultragoal.openai.typed-output.v1";

#[derive(Debug)]
pub(crate) struct OutputCommand {
    receipt: PathBuf,
    call_receipt: PathBuf,
    parser_schema_id: String,
    parsed_output_digest: String,
}

pub(crate) fn parse(raw: &[String]) -> OutputCommand {
    OutputCommand {
        receipt: super::opt_path(raw, "--receipt")
            .unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT)),
        call_receipt: super::opt_path(raw, "--call-receipt")
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CALL_RECEIPT)),
        parser_schema_id: super::opt_string(raw, "--parser-schema-id")
            .unwrap_or_else(|| DEFAULT_PARSER_SCHEMA.to_string()),
        parsed_output_digest: super::opt_string(raw, "--parsed-output-digest")
            .unwrap_or_else(|| crate::digest::ZERO.to_string()),
    }
}

pub(crate) fn run(root: &Path, command: &OutputCommand) -> Result<i32, String> {
    let receipt = build_output_receipt(root, command)?;
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

pub(crate) fn build_output_receipt(root: &Path, command: &OutputCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let call_path = rel_path(&command.call_receipt);
    let call_abs = if command.call_receipt.is_absolute() {
        command.call_receipt.clone()
    } else {
        root.join(&command.call_receipt)
    };
    let call_receipt = crate::json_boundary::read_json(&call_abs).unwrap_or(Value::Null);
    let failures = output_failures(&candidate, command, &call_receipt);
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let receipt_rel = rel_path(&command.receipt);
    let observability = observability(root, &receipt_rel, status, &failures)?;
    let value = json!({
        "schema": "harness-ultragoal.openai-model-output-authority.v1",
        "status": status,
        "candidate_digest": candidate,
        "provider_mode": str_field(&call_receipt, "provider_mode"),
        "model_identity": str_field(&call_receipt, "model_identity"),
        "source_call_receipt_path": call_path,
        "source_call_receipt_digest": crate::digest::file(&call_abs).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        "prompt_input_digest": str_field(&call_receipt, "prompt_input_digest"),
        "raw_output_digest": str_field(&call_receipt, "output_digest"),
        "parser_schema_id": command.parser_schema_id,
        "parsed_output_digest": command.parsed_output_digest,
        "authority_state": "typed_observation_not_claim_authority",
        "redaction_status": "pass",
        "observability_receipt": observability,
        "supported_claims": if status == "pass" {
            vec!["openai_model_output_typed_parse_boundary"]
        } else {
            Vec::<&str>::new()
        },
        "blocked_claims": super::config::blocked_claims(),
        "claim_impact": if status == "pass" {
            "supports_typed_model_output_observation_only"
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

pub(crate) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if receipt.get("schema").and_then(Value::as_str)
        != Some("harness-ultragoal.openai-model-output-authority.v1")
    {
        out.push("openai_model_output_receipt_wrong_schema".to_string());
    }
    if receipt.get("status").and_then(Value::as_str) != Some("pass") {
        out.push("openai_model_output_receipt_not_passing".to_string());
    }
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    if receipt.get("candidate_digest").and_then(Value::as_str) != Some(candidate.as_str()) {
        out.push("openai_model_output_receipt_candidate_digest_mismatch".to_string());
    }
    if !valid_digest(receipt, "source_call_receipt_digest")
        || !valid_digest(receipt, "parsed_output_digest")
    {
        out.push("openai_model_output_receipt_digest_invalid".to_string());
    }
    if receipt.get("authority_state").and_then(Value::as_str)
        != Some("typed_observation_not_claim_authority")
    {
        out.push("openai_model_output_receipt_authority_overbroad".to_string());
    }
    if !super::policy::contains_secret_shape(receipt)
        && receipt.get("redaction_status").and_then(Value::as_str) == Some("pass")
    {
        return out;
    }
    out.push("openai_model_output_receipt_secret_leak_or_redaction_failure".to_string());
    out
}

fn output_failures(candidate: &str, command: &OutputCommand, call_receipt: &Value) -> Vec<String> {
    let mut failures = Vec::new();
    if call_receipt.get("status").and_then(Value::as_str) != Some("pass") {
        failures.push("openai_output_source_call_receipt_not_passing".to_string());
    }
    if call_receipt.get("candidate_digest").and_then(Value::as_str) != Some(candidate) {
        failures.push("openai_output_source_call_candidate_mismatch".to_string());
    }
    if command.parser_schema_id.trim().is_empty() {
        failures.push("openai_output_parser_schema_missing".to_string());
    }
    if !is_sha256(&command.parsed_output_digest) {
        failures.push("openai_output_parsed_digest_invalid".to_string());
    }
    failures
}

fn observability(
    root: &Path,
    receipt_rel: &str,
    status: &str,
    failures: &[String],
) -> Result<Value, String> {
    crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal openai",
            subcommand: "output prove",
            operation: "openai.output.prove",
            surface: "source",
            law_id: super::LAW_ID,
            check_id: super::LAW_ID,
            claim_id: "openai_model_output_typed_parse_boundary",
            artifact_path: "schemas/openai-model-output-authority.schema.json",
            receipt_path: receipt_rel,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "schema_invalid"
            },
            why_failed: if failures.is_empty() {
                "none"
            } else {
                "see_failures"
            },
            where_failed: "validation_artifacts/openai/call-receipt.json",
            next_repair: "mint current OpenAI call receipt and typed output parse digest",
            claim_impact: "blocks_model_output_claim_authority",
            blocked_claims: super::config::blocked_claims(),
            supported_claims: if status == "pass" {
                vec!["openai_model_output_typed_parse_boundary".to_string()]
            } else {
                Vec::new()
            },
            runtime: None,
            emit: true,
        },
    )
}

fn secret_leak_receipt(mut value: Value) -> Value {
    value["status"] = json!("fail");
    value["redaction_status"] = json!("fail");
    value["supported_claims"] = json!([]);
    value["claim_impact"] = json!("withheld_or_blocked");
    value["failures"] = json!(["openai_model_output_receipt_secret_shape_detected"]);
    value
}

fn print_receipt(receipt: &Path, value: &Value) {
    println!(
        "ultragoal-openai-output {} candidate={} receipt={} run_id={} correlation_id={} claim_impact={}",
        value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("fail"),
        value
            .get("candidate_digest")
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        receipt.display(),
        value
            .get("observability_receipt")
            .and_then(|obs| obs.get("run_id"))
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("observability_receipt")
            .and_then(|obs| obs.get("correlation_id"))
            .and_then(Value::as_str)
            .unwrap_or("<missing>"),
        value
            .get("claim_impact")
            .and_then(Value::as_str)
            .unwrap_or("<missing>")
    );
}

fn rel_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn valid_digest(receipt: &Value, key: &str) -> bool {
    receipt
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(is_sha256)
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|tail| tail.len() == 64 && tail.chars().all(|ch| ch.is_ascii_hexdigit()))
}
