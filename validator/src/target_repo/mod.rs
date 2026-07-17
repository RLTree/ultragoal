pub(crate) mod artifact_refs;
mod baseline;
mod baseline_mode;
mod check_gate;
mod observability;
pub(crate) mod product;
mod receipt;
pub(crate) mod safe_fs;
pub(crate) mod worktree_env_contract;

use serde_json::Value;
use std::path::Path;

const TARGET_CHECK_IDS: &[&str] = &[
    "agent-surface",
    "standards-enforcement",
    "check-gate",
    "mode-provenance",
    "retrofit-backlog",
    "runtime-legibility",
    "observability-stack",
    "product-cohesion",
];

pub fn audit_target_repo(
    repo: &Path,
    mode: &str,
    command_text: &str,
    validator_artifacts: &[Value],
    require_observability: bool,
    require_product_cohesion: bool,
    display_root: Option<&Path>,
) -> (Value, i32) {
    let mut checks = serde_json::Map::new();
    baseline::baseline_checks(repo, &mut checks);
    baseline::agent_surface_check(repo, &mut checks);
    baseline::standards_enforcement_check(repo, &mut checks);
    let gate = check_gate::run(repo);
    checks.insert("check-gate".to_string(), gate.check.clone());
    baseline_mode::mode_checks(repo, mode, &mut checks);
    baseline::runtime_check(repo, &mut checks);
    observability::check(repo, &gate.markers, require_observability, &mut checks);
    product::cohesion::check(repo, &gate.markers, require_product_cohesion, &mut checks);
    let status = receipt::overall_status(&checks);
    let display_repo = display_repo(repo, display_root);
    let output = receipt::target_receipt(receipt::TargetReceiptInput {
        display_repo: &display_repo,
        mode,
        status: &status,
        checks,
        command_text,
        command: display_command(gate.command, &display_repo),
        validator_artifacts,
        markers: gate.markers,
        require_observability,
        require_product_cohesion,
    });
    let code = if status == "pass" { 0 } else { 1 };
    (output, code)
}

fn display_repo(repo: &Path, display_root: Option<&Path>) -> String {
    display_root
        .and_then(|root| {
            let root = root.canonicalize().ok()?;
            let repo = repo.canonicalize().ok()?;
            repo.strip_prefix(root).ok().map(|rel| rel.to_path_buf())
        })
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| "<target-repo>".to_string())
}

fn display_command(mut command: Value, display_repo: &str) -> Value {
    if let Some(obj) = command.as_object_mut() {
        obj.insert("cwd".to_string(), Value::String(display_repo.to_string()));
    }
    command
}

pub fn target_receipt_errors(target_receipt: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let checks = target_receipt.get("checks").and_then(Value::as_object);
    for id in baseline::check_ids()
        .into_iter()
        .chain(TARGET_CHECK_IDS.iter().map(|s| s.to_string()))
    {
        if checks.is_none_or(|map| !map.contains_key(&id)) {
            errors.push(format!("missing target checks: {id}"));
        }
    }
    if target_receipt
        .get("repo_fingerprint")
        .and_then(Value::as_str)
        != Some(&receipt::canonical_fingerprint(target_receipt))
    {
        errors.push("target repo fingerprint mismatch".to_string());
    }
    if target_receipt
        .pointer("/checks/check-gate/status")
        .and_then(Value::as_str)
        == Some("pass")
    {
        gate_receipt_errors(target_receipt, &mut errors);
    }
    errors
}

pub(crate) fn row(repo: &Path, status: &str, detail: &str, rel: Option<&str>) -> Value {
    receipt::row(repo, status, detail, rel)
}

fn gate_receipt_errors(target_receipt: &Value, errors: &mut Vec<String>) {
    let markers = target_receipt
        .get("gate_markers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    for required in check_gate::REQUIRED_MARKERS {
        if !markers.contains(required) {
            errors.push("target gate missing required markers".to_string());
        }
    }
    if target_receipt.pointer("/command/stdout").is_none()
        || target_receipt.pointer("/command/stderr").is_none()
    {
        errors.push("target gate missing stdout/stderr digests".to_string());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn display_contracts_are_fail_closed_and_stable() {
        assert_eq!(
            super::display_repo(Path::new("/missing/repo"), Some(Path::new("/missing/root"))),
            "<target-repo>"
        );
        assert_eq!(
            super::display_command(json!("not-object"), "repo"),
            json!("not-object")
        );
    }
}
