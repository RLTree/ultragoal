use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub(crate) struct RoutineCommand {
    pub(crate) receipt: PathBuf,
    pub(crate) target_repo: Option<PathBuf>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<RoutineCommand>, String> {
    match raw {
        [a, b, ..] if a == "routine" && b == "check" => Ok(Some(RoutineCommand {
            receipt: opt_path(&raw[2..], "--receipt").unwrap_or_else(|| {
                PathBuf::from("validation_artifacts/cli/routine-check-receipt.json")
            }),
            target_repo: opt_path(&raw[2..], "--target-repo"),
        })),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &RoutineCommand) -> Result<i32, String> {
    let value = receipt(root, command)?;
    let out = output_path(root, &command.receipt)?;
    crate::json_boundary::write_json(&out, &value)?;
    println!(
        "ultragoal-routine {} candidate={} receipt={}",
        value["status"],
        value["candidate_digest"],
        out.display()
    );
    Ok(i32::from(
        value.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn receipt(root: &Path, command: &RoutineCommand) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    let script = std::fs::read_to_string(root.join("scripts/check")).unwrap_or_default();
    let failures = surface_failures(crate::cli::usage::text(), &script);
    Ok(json!({
        "schema": "harness-ultragoal.routine-validation-receipt.v1",
        "operation": "routine.check",
        "status": if failures.is_empty() { "pass" } else { "fail" },
        "candidate_digest": candidate,
        "target_repo": command.target_repo.as_ref().map(|p| p.display().to_string()),
        "entrypoints": {
            "routine": "ultragoal routine check --receipt validation_artifacts/cli/routine-check-receipt.json",
            "fit_repo": "ultragoal fit-repo prove --receipt-dir validation_artifacts/harness",
            "target_repo": "ultragoal target-repo audit --surface-root <repo> --receipt <path>",
            "scripts_check": "narrow_helper_or_delegate_only"
        },
        "surface_checks": {
            "routine_entrypoint": failures.iter().all(|f| !f.contains("routine_entrypoint")),
            "help_navigable": failures.iter().all(|f| !f.contains("help_not_navigable")),
            "fit_repo_visible": failures.iter().all(|f| !f.contains("fit_repo_hidden")),
            "target_repo_visible": failures.iter().all(|f| !f.contains("target_repo_hidden")),
            "scripts_check_ceiling": failures.iter().all(|f| !f.contains("scripts_check"))
        },
        "claim_ceiling": "routine_usability_only_not_readiness",
        "supported_claim_classes": ["routine_usability"],
        "blocked_claim_classes": [
            "product_readiness",
            "readiness",
            "release",
            "completion",
            "final_packet_correctness",
            "update_goal_eligibility",
            "install_cache_parity",
            "app_registry_or_reviewer_exposure"
        ],
        "failures": failures
    }))
}

pub(crate) fn surface_failures(help: &str, script: &str) -> Vec<String> {
    let mut out = Vec::new();
    if !help.contains("routine check") {
        out.push("routine_entrypoint_missing".to_string());
    }
    if help.lines().count() < 12
        || !help.contains("Routine validation")
        || !help.contains("unsupported claims")
    {
        out.push("routine_help_not_navigable".to_string());
    }
    if help.contains("source audit") && !help.contains("routine check") {
        out.push("routine_leaf_only_validation_substitution".to_string());
    }
    if !help.contains("fit-repo prove") || !help.contains("First plugin-activated repo path") {
        out.push("routine_fit_repo_hidden".to_string());
    }
    if !help.contains("target-repo audit") || !help.contains("Target repo path") {
        out.push("routine_target_repo_hidden".to_string());
    }
    let delegates = script.contains("routine check");
    let narrow = script.contains("narrow-helper ceiling") && script.contains("unsupported claims");
    if !delegates && !narrow {
        out.push("routine_scripts_check_claim_ceiling_missing".to_string());
    }
    out
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}

fn output_path(root: &Path, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Err("routine receipt path must be root-relative".to_string());
    }
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!(
            "{}: routine receipt path escapes root",
            path.display()
        ));
    }
    Ok(root.join(path))
}
