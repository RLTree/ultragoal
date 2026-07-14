use super::{PromptfooCommand, registry};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn build_receipt(root: &Path, command: &PromptfooCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let mut failures = Vec::new();
    let package_json = read_json(root, super::PACKAGE_JSON, &mut failures);
    let providers = read_json(root, super::PROVIDER_REGISTRY, &mut failures);
    let suites = read_json(root, super::SUITE_REGISTRY, &mut failures);
    let version = observed_version(root, &command.promptfoo_bin, &mut failures);
    failures.extend(registry::package_failures(&package_json));
    failures.extend(registry::provider_failures(&providers));
    failures.extend(registry::suite_failures(&suites));
    if version != super::PROMPTFOO_VERSION {
        failures.push("promptfoo_cli_version_mismatch".to_string());
    }
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let failure_text = failure_text(&failures);
    let receipt_rel = command.receipt.to_string_lossy().replace('\\', "/");
    let observability = observability(root, status, &failure_text, &receipt_rel)?;
    let mut value = json!({
        "schema": super::RECEIPT_SCHEMA,
        "status": status,
        "candidate_digest": candidate,
        "package_json_digest": digest_or_zero(root, super::PACKAGE_JSON),
        "pnpm_lock_digest": digest_or_zero(root, super::PNPM_LOCK),
        "pnpm_workspace_digest": digest_or_zero(root, super::PNPM_WORKSPACE),
        "promptfoo_version": version,
        "promptfoo_binary": command.promptfoo_bin.to_string_lossy().replace('\\', "/"),
        "install_lifecycle_status": "build_scripts_not_approved_no_eval_execution_claimed",
        "provider_registry_path": super::PROVIDER_REGISTRY,
        "provider_registry_digest": digest_or_zero(root, super::PROVIDER_REGISTRY),
        "suite_registry_path": super::SUITE_REGISTRY,
        "suite_registry_digest": digest_or_zero(root, super::SUITE_REGISTRY),
        "provider_modes": registry::provider_modes(&providers),
        "suite_ids": registry::suite_ids(&suites),
        "raw_promptfoo_authority": "observation_only_until_cli_parsed_receipt",
        "observability_receipt": observability,
        "supported_claims": if status == "pass" {
            vec!["promptfoo_adapter_provider_separation"]
        } else {
            Vec::<&str>::new()
        },
        "blocked_claims": blocked_claims(),
        "claim_impact": if status == "pass" {
            "supports_promptfoo_adapter_setup_boundary_only_not_eval_product_or_readiness_claims"
        } else {
            "withheld_or_blocked"
        },
        "failures": failures
    });
    if crate::cli::openai::policy::contains_secret_shape(&value) {
        value["status"] = json!("fail");
        value["failures"] = json!(["promptfoo_receipt_secret_shape_detected"]);
        value["supported_claims"] = json!([]);
        value["claim_impact"] = json!("withheld_or_blocked");
    }
    Ok(value)
}

pub(super) fn receipt_failures(root: &Path, receipt: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let candidate = crate::package::inventory::package_digest(root).unwrap_or_default();
    registry::require_str(receipt, "schema", super::RECEIPT_SCHEMA, &mut out);
    registry::require_str(receipt, "status", "pass", &mut out);
    registry::require_str(receipt, "candidate_digest", &candidate, &mut out);
    registry::require_str(
        receipt,
        "promptfoo_version",
        super::PROMPTFOO_VERSION,
        &mut out,
    );
    registry::require_str(
        receipt,
        "raw_promptfoo_authority",
        "observation_only_until_cli_parsed_receipt",
        &mut out,
    );
    for (key, rel) in [
        ("package_json_digest", super::PACKAGE_JSON),
        ("pnpm_lock_digest", super::PNPM_LOCK),
        ("pnpm_workspace_digest", super::PNPM_WORKSPACE),
        ("provider_registry_digest", super::PROVIDER_REGISTRY),
        ("suite_registry_digest", super::SUITE_REGISTRY),
    ] {
        require_digest(root, receipt, key, rel, &mut out);
    }
    if !blocks_required_claims(receipt) {
        out.push("promptfoo_receipt_missing_claim_blockers".to_string());
    }
    check_observability(receipt, &candidate, &mut out);
    if crate::cli::openai::policy::contains_secret_shape(receipt) {
        out.push("promptfoo_receipt_secret_shape_detected".to_string());
    }
    out
}

fn observability(root: &Path, status: &str, why: &str, receipt: &str) -> Result<Value, String> {
    crate::cli::observe::telemetry::command_receipt(
        root,
        crate::cli::observe::telemetry::CommandTelemetry {
            command: "ultragoal promptfoo",
            subcommand: "prove",
            operation: "promptfoo.prove",
            surface: "source",
            law_id: super::LAW_ID,
            check_id: super::LAW_ID,
            claim_id: "promptfoo_adapter_provider_separation",
            artifact_path: "schemas/promptfoo-adapter-receipt.schema.json",
            receipt_path: receipt,
            status,
            failure_class: if status == "pass" {
                "none"
            } else {
                "schema_invalid"
            },
            why_failed: why,
            where_failed: "promptfoo adapter package/provider registries",
            next_repair: if status == "pass" {
                "none"
            } else {
                "repair package.json, pnpm-lock.yaml, promptfoo registries, or pinned CLI install"
            },
            claim_impact: if status == "pass" {
                "supports_promptfoo_adapter_setup_boundary_only"
            } else {
                "blocks_promptfoo_eval_claims"
            },
            blocked_claims: blocked_claim_strings(),
            supported_claims: if status == "pass" {
                vec!["promptfoo_adapter_provider_separation".to_string()]
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
        failures.push(format!("promptfoo_json_missing_or_malformed:{rel}:{err}"));
        Value::Null
    })
}

fn observed_version(root: &Path, bin: &Path, failures: &mut Vec<String>) -> String {
    let output = std::process::Command::new(super::resolve(root, bin))
        .arg("--version")
        .output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .unwrap_or("")
            .trim()
            .to_string(),
        Ok(output) => {
            failures.push(format!(
                "promptfoo_version_command_failed:{}",
                output.status
            ));
            String::new()
        }
        Err(err) => {
            failures.push(format!("promptfoo_version_command_unavailable:{err}"));
            String::new()
        }
    }
}

fn digest_or_zero(root: &Path, rel: &str) -> String {
    crate::digest::file(&root.join(rel)).unwrap_or_else(|_| crate::digest::ZERO.to_string())
}

fn require_digest(root: &Path, receipt: &Value, key: &str, rel: &str, out: &mut Vec<String>) {
    if receipt.get(key).and_then(Value::as_str) != Some(digest_or_zero(root, rel).as_str()) {
        out.push(format!("promptfoo_receipt_digest_mismatch:{key}"));
    }
}

fn check_observability(receipt: &Value, candidate: &str, out: &mut Vec<String>) {
    let obs = receipt.get("observability_receipt").unwrap_or(&Value::Null);
    if obs.get("schema").and_then(Value::as_str)
        != Some(crate::cli::observe::command::RECEIPT_SCHEMA)
        || obs.get("status").and_then(Value::as_str) != Some("pass")
        || obs.get("candidate_digest").and_then(Value::as_str) != Some(candidate)
        || obs.get("law_id").and_then(Value::as_str) != Some(super::LAW_ID)
        || obs.get("check_id").and_then(Value::as_str) != Some(super::LAW_ID)
    {
        out.push("promptfoo_receipt_observability_binding_invalid".to_string());
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
        "product_success",
        "product_fitness",
        "live_model_authority",
        "promptfoo_raw_pass_authority",
        "reviewer_exposure",
        "app_registry_exposure",
        "eval_result_claim",
    ]
}

fn blocked_claim_strings() -> Vec<String> {
    blocked_claims().into_iter().map(str::to_string).collect()
}

fn failure_text(failures: &[String]) -> String {
    if failures.is_empty() {
        "none".to_string()
    } else {
        failures.join("; ")
    }
}
