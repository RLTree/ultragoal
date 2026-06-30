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

#[cfg(test)]
mod tests {
    use super::{RoutineCommand, receipt, surface_failures};
    use serde_json::json;
    use std::path::PathBuf;

    fn root(label: &str, script: &str) -> PathBuf {
        let root = crate::self_tests::boundaries::support::temp_root(label);
        std::fs::create_dir_all(root.join("scripts")).expect("scripts");
        std::fs::write(root.join("scripts/check"), script).expect("script");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["scripts/check"]}),
        )
        .expect("manifest");
        root
    }

    #[test]
    fn routine_receipt_passes_for_discoverable_entrypoints_and_narrow_script() {
        let root = root(
            "routine-pass",
            "narrow-helper ceiling; unsupported claims; readiness blocked",
        );
        let command = RoutineCommand {
            receipt: PathBuf::from("validation_artifacts/cli/routine.json"),
            target_repo: Some(PathBuf::from("target-repo")),
        };
        let value = receipt(&root, &command).expect("receipt");
        assert_eq!(value["status"], "pass");
        assert_eq!(
            value["supported_claim_classes"],
            json!(["routine_usability"])
        );
        assert!(
            value["blocked_claim_classes"]
                .as_array()
                .unwrap()
                .contains(&json!("readiness"))
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn routine_red_edges_reject_missing_help_and_script_surfaces() {
        let failures = surface_failures("usage: ultragoal source audit --receipt x", "");
        for expected in [
            "routine_entrypoint_missing",
            "routine_help_not_navigable",
            "routine_leaf_only_validation_substitution",
            "routine_fit_repo_hidden",
            "routine_target_repo_hidden",
            "routine_scripts_check_claim_ceiling_missing",
        ] {
            assert!(failures.iter().any(|item| item == expected), "{expected}");
        }
    }
}
