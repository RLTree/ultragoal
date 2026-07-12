use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

mod receipt_status;
#[cfg(test)]
mod tests;

pub(crate) use receipt_status::receipt_state;

#[derive(Debug)]
pub(crate) struct CurrentStateCommand {
    pub(crate) json: bool,
    pub(crate) receipt: Option<PathBuf>,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<CurrentStateCommand>, String> {
    if raw.first().map(String::as_str) != Some("current-state") {
        return Ok(None);
    }
    Ok(Some(CurrentStateCommand {
        json: raw.iter().any(|arg| arg == "--json"),
        receipt: opt_path(&raw[1..], "--receipt"),
    }))
}

pub(crate) fn run(root: &Path, command: &CurrentStateCommand) -> Result<i32, String> {
    let state = snapshot(root)?;
    if let Some(relative) = command.receipt.as_ref() {
        let receipt =
            crate::output_path::claim_artifact_path(root, relative, "current state receipt")?;
        crate::json_boundary::write_json(&receipt, &state)?;
    }
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&state).expect("json value")
        );
    } else {
        print_summary(&state, command.receipt.as_deref());
    }
    Ok(i32::from(
        state.get("status").and_then(Value::as_str) != Some("pass"),
    ))
}

pub(crate) fn snapshot(root: &Path) -> Result<Value, String> {
    let candidate = crate::package::inventory::package_digest(root)?;
    Ok(snapshot_for_candidate(root, candidate))
}

pub(crate) fn snapshot_for_candidate(root: &Path, candidate: String) -> Value {
    let git = git_status(root);
    let coverage = receipt_state(
        root,
        "validation_artifacts/coverage/coverage-receipt.json",
        &candidate,
    );
    let source_audit = receipt_state(
        root,
        "validation_artifacts/ultragoal-audit/validator-receipt.json",
        &candidate,
    );
    let red_report = receipt_state(
        root,
        "validation_artifacts/ultragoal-audit/red-fixture-report.json",
        &candidate,
    );
    let observability_control_board = observability_control_board(root);
    let first_blocker = first_blocker(
        &observability_control_board,
        &coverage,
        &source_audit,
        &red_report,
    );
    json!({
        "schema": "harness-ultragoal.current-state.v1",
        "status": if first_blocker["id"].as_str() == Some("none") { "pass" } else { "fail" },
        "candidate_digest": candidate,
        "active_stage": "custom_tooling_prerequisite",
        "git": git,
        "coverage": coverage,
        "source_audit": source_audit,
        "red_report": red_report,
        "observability_control_board": observability_control_board,
        "first_blocker": first_blocker,
        "next_repair": first_blocker["next_repair"],
        "narrow_rerun": first_blocker["narrow_rerun"],
        "claim_ceiling": "source_local_custom_tooling_prerequisite_only",
        "source_receipts": [
            "validation_artifacts/coverage/coverage-receipt.json",
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
            "validation_artifacts/ultragoal-audit/red-fixture-report.json"
        ],
        "unavailable_dependencies": [
            crate::audit::observability::command_inventory_failures(root)[0]
        ]
    })
}

fn observability_control_board(root: &Path) -> Value {
    let blocker = crate::audit::observability::command_inventory_failures(root)
        .into_iter()
        .next()
        .unwrap_or_else(|| "HCT-OBSERVE successor catalog unavailable/not adopted".to_string());
    json!({
        "status": "unavailable",
        "capability": "HCT-OBSERVE",
        "adoption_status": "not_adopted",
        "failure_class": "hct_observe_successor_catalog_unavailable",
        "why_failed": blocker,
        "first_incomplete": {
            "family": "successor_catalog",
            "id": "HCT-OBSERVE",
            "observability_status": "unavailable",
            "next_unobservable_surface": "typed candidate-bound successor catalog"
        }
    })
}

fn first_blocker(board: &Value, coverage: &Value, audit: &Value, red: &Value) -> Value {
    if board.get("status").and_then(Value::as_str) != Some("observable") {
        if board.get("status").and_then(Value::as_str) == Some("unavailable") {
            return json!({
                "id": "HCT-OBSERVE",
                "surface": "successor observability catalog",
                "failure_class": "hct_observe_successor_catalog_unavailable",
                "why_failed": text(board, "why_failed", "HCT-OBSERVE successor catalog unavailable/not adopted"),
                "next_repair": "implement and adopt the typed candidate-bound HCT-OBSERVE successor catalog",
                "narrow_rerun": "ultragoal current-state --json",
                "broad_rerun": "source audit only after HCT-OBSERVE adoption and narrow verification"
            });
        }
        let incomplete = board
            .get("first_incomplete")
            .cloned()
            .unwrap_or(Value::Null);
        let id = text(&incomplete, "id", "observability_control_board");
        let (next_repair, narrow_rerun) = observability_repair(id);
        return json!({
            "id": id,
            "surface": text(&incomplete, "family", "observability"),
            "failure_class": "unobservable_authority_surface",
            "why_failed": format!("observability control board is {}", text(board, "status", "missing")),
            "next_repair": next_repair,
            "narrow_rerun": narrow_rerun,
            "broad_rerun": "source audit once after narrow observable proof passes"
        });
    }
    for receipt in [coverage, audit, red] {
        if !receipt
            .get("current")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || receipt.get("status").and_then(Value::as_str) != Some("pass")
        {
            return json!({
                "id": receipt.get("path").and_then(Value::as_str).unwrap_or("receipt"),
                "surface": "source-local receipt",
                "failure_class": text(receipt, "failure_class", "stale_or_missing_source_local_authority"),
                "why_failed": format!(
                    "{} status={} current={}",
                    receipt.get("path").and_then(Value::as_str).unwrap_or("receipt"),
                    text(receipt, "status", "unknown"),
                    receipt.get("current").and_then(Value::as_bool).unwrap_or(false)
                ),
                "next_repair": receipt_repair(receipt),
                "narrow_rerun": receipt_rerun(receipt),
                "broad_rerun": "source audit once after narrow proof passes"
            });
        }
    }
    json!({"id": "none", "next_repair": "none", "narrow_rerun": "none"})
}

fn observability_repair(id: &str) -> (String, String) {
    let narrow_rerun = format!("ultragoal observe command-roundtrip --command \"{id}\"");
    if crate::audit::observability::specs::command(id).is_some() {
        (
            format!(
                "run {narrow_rerun} and inspect same-candidate logs metrics traces and explain proof"
            ),
            narrow_rerun,
        )
    } else {
        (
            format!(
                "extend CommandObservabilitySpec or SurfaceObservabilitySpec for {id}, then run {narrow_rerun} and inspect same-candidate query proof"
            ),
            narrow_rerun,
        )
    }
}

fn receipt_repair(receipt: &Value) -> String {
    match receipt.get("path").and_then(Value::as_str).unwrap_or("") {
        "validation_artifacts/coverage/coverage-receipt.json" => {
            "rerun strict coverage proof for the current candidate or leave coverage claim blocked"
                .to_string()
        }
        "validation_artifacts/ultragoal-audit/validator-receipt.json" => {
            "rerun source audit for the current candidate after the narrow source repair passes"
                .to_string()
        }
        "validation_artifacts/ultragoal-audit/red-fixture-report.json" => {
            "rerun red fixture report for the current candidate after the implicated fixture or validator repair"
                .to_string()
        }
        path => format!("rebuild typed authority for {path} on the current candidate digest"),
    }
}

fn receipt_rerun(receipt: &Value) -> String {
    match receipt.get("path").and_then(Value::as_str).unwrap_or("") {
        "validation_artifacts/coverage/coverage-receipt.json" => {
            "ultragoal coverage prove --receipt validation_artifacts/coverage/coverage-receipt.json"
        }
        "validation_artifacts/ultragoal-audit/validator-receipt.json" => {
            "ultragoal source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json"
        }
        "validation_artifacts/ultragoal-audit/red-fixture-report.json" => {
            "ultragoal red fixture report --report validation_artifacts/ultragoal-audit/red-fixture-report.json"
        }
        _ => "ultragoal current-state --json",
    }
    .to_string()
}

fn git_status(root: &Path) -> Value {
    match Command::new("git")
        .arg("status")
        .arg("--short")
        .arg("--untracked-files=all")
        .current_dir(root)
        .output()
    {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            let files = text.lines().map(ToString::to_string).collect::<Vec<_>>();
            json!({"dirty": !files.is_empty(), "files": files})
        }
        Err(err) => json!({"dirty": true, "files": [], "error": err.to_string()}),
    }
}

fn print_summary(state: &Value, receipt: Option<&Path>) {
    let blocker = state.get("first_blocker").unwrap_or(&Value::Null);
    println!(
        "ultragoal-current-state {} candidate={} receipt={} first_blocker={} why={} next_repair={} narrow_rerun='{}' claim_ceiling='{}'",
        text(state, "status", "fail"),
        text(state, "candidate_digest", "<missing>"),
        receipt
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none-read-only".to_string()),
        text(blocker, "id", "unknown"),
        text(blocker, "why_failed", "unknown"),
        text(state, "next_repair", "unknown"),
        text(state, "narrow_rerun", "unknown"),
        text(state, "claim_ceiling", "unknown")
    );
}

fn text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

fn opt_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
}
