use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(crate) struct CurrentStateCommand {
    pub(crate) json: bool,
    pub(crate) receipt: PathBuf,
}

pub(crate) fn parse(raw: &[String]) -> Result<Option<CurrentStateCommand>, String> {
    if raw.first().map(String::as_str) != Some("current-state") {
        return Ok(None);
    }
    Ok(Some(CurrentStateCommand {
        json: raw.iter().any(|arg| arg == "--json"),
        receipt: opt_path(&raw[1..], "--receipt")
            .unwrap_or_else(|| PathBuf::from("validation_artifacts/current-state.json")),
    }))
}

pub(crate) fn run(root: &Path, command: &CurrentStateCommand) -> Result<i32, String> {
    let state = snapshot(root)?;
    let receipt =
        crate::output_path::claim_artifact_path(root, &command.receipt, "current state receipt")?;
    crate::json_boundary::write_json(&receipt, &state)?;
    if command.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&state).expect("json value")
        );
    } else {
        print_summary(&state, &command.receipt);
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
        "git": git,
        "coverage": coverage,
        "source_audit": source_audit,
        "red_report": red_report,
        "observability_control_board": observability_control_board,
        "first_blocker": first_blocker,
        "next_repair": first_blocker["next_repair"],
        "narrow_rerun": first_blocker["narrow_rerun"],
        "claim_ceiling": "source-local only; no readiness release completion final-packet install/cache registry reviewer or update_goal claim",
        "source_receipts": [
            "validation_artifacts/coverage/coverage-receipt.json",
            "validation_artifacts/ultragoal-audit/validator-receipt.json",
            "validation_artifacts/ultragoal-audit/red-fixture-report.json",
            "docs/generated/observability/command-inventory.json"
        ]
    })
}

fn receipt_state(root: &Path, rel: &str, candidate: &str) -> Value {
    let path = root.join(rel);
    let Ok(value) = crate::json_boundary::read_json(&path) else {
        return json!({"path": rel, "status": "missing", "current": false});
    };
    let observed = digest_field(&value).unwrap_or("missing");
    json!({
        "path": rel,
        "status": value.get("status").and_then(Value::as_str).unwrap_or("unknown"),
        "current": observed == candidate,
        "target_digest": observed,
        "first_detail": value.pointer("/failures/0/detail")
            .or_else(|| value.pointer("/details/0"))
            .and_then(Value::as_str)
            .unwrap_or("none")
    })
}

fn digest_field(value: &Value) -> Option<&str> {
    value
        .pointer("/target_revision/value")
        .or_else(|| value.get("target_digest"))
        .or_else(|| value.get("candidate_digest"))
        .and_then(Value::as_str)
}

fn observability_control_board(root: &Path) -> Value {
    let value = crate::json_boundary::read_json(
        &root.join("docs/generated/observability/command-inventory.json"),
    )
    .unwrap_or(Value::Null);
    value
        .get("observability_control_board")
        .cloned()
        .unwrap_or_else(|| json!({"status": "missing"}))
}

fn first_blocker(board: &Value, coverage: &Value, audit: &Value, red: &Value) -> Value {
    if board.get("status").and_then(Value::as_str) != Some("observable") {
        let incomplete = board
            .get("first_incomplete")
            .cloned()
            .unwrap_or(Value::Null);
        let id = text(&incomplete, "id", "observability_control_board");
        let (next_repair, narrow_rerun) = observability_repair(id);
        return json!({
            "id": id,
            "surface": text(&incomplete, "family", "observability"),
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
                "why_failed": format!(
                    "{} status={} current={}",
                    receipt.get("path").and_then(Value::as_str).unwrap_or("receipt"),
                    text(receipt, "status", "unknown"),
                    receipt.get("current").and_then(Value::as_bool).unwrap_or(false)
                ),
                "next_repair": "repair the named source-local receipt on the current digest",
                "narrow_rerun": "rerun the failing receipt command only",
                "broad_rerun": "source audit once after narrow proof passes"
            });
        }
    }
    json!({"id": "none", "next_repair": "none", "narrow_rerun": "none"})
}

fn observability_repair(id: &str) -> (String, String) {
    let narrow_rerun = format!("ultragoal observe fit --command \"{id}\"");
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

fn print_summary(state: &Value, receipt: &Path) {
    let blocker = state.get("first_blocker").unwrap_or(&Value::Null);
    println!(
        "ultragoal-current-state {} candidate={} receipt={} first_blocker={} why={} next_repair={} narrow_rerun='{}' claim_ceiling='{}'",
        text(state, "status", "fail"),
        text(state, "candidate_digest", "<missing>"),
        receipt.display(),
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
