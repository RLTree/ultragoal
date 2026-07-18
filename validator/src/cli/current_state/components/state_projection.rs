use super::*;

#[cfg(test)]
#[derive(Debug)]
pub(crate) struct CurrentStateCommand {
    pub(crate) json: bool,
    pub(crate) receipt: Option<PathBuf>,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<CurrentStateCommand>, String> {
    if raw.first().map(String::as_str) != Some("current-state") {
        return Ok(None);
    }
    Ok(Some(CurrentStateCommand {
        json: raw.iter().any(|arg| arg == "--json"),
        receipt: opt_path(&raw[1..], "--receipt"),
    }))
}

#[cfg(test)]
pub(crate) fn run(root: &Path, command: &CurrentStateCommand) -> Result<i32, String> {
    let state = snapshot(root)?;
    if let Some(relative) = command.receipt.as_ref() {
        let receipt =
            crate::output_path::claim_artifact_path(root, relative, "current state receipt")?;
        crate::self_tests::boundaries::workspace_fixtures::write_json(&receipt, &state)?;
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

pub(crate) fn observability_control_board(root: &Path) -> Value {
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

pub(crate) fn first_blocker(board: &Value, coverage: &Value, audit: &Value, red: &Value) -> Value {
    if board.get("status").and_then(Value::as_str) != Some("observable") {
        if board.get("status").and_then(Value::as_str) == Some("unavailable") {
            return json!({
                "id": "HCT-OBSERVE",
                "surface": "successor observability catalog",
                "failure_class": "hct_observe_successor_catalog_unavailable",
                "why_failed": projection_text(board, "why_failed", "HCT-OBSERVE successor catalog unavailable/not adopted"),
                "next_repair": "implement and adopt the typed candidate-bound HCT-OBSERVE successor catalog",
                "narrow_rerun": "ultragoal current-state --json",
                "broad_rerun": "source audit only after HCT-OBSERVE adoption and narrow verification"
            });
        }
        let incomplete = board
            .get("first_incomplete")
            .cloned()
            .unwrap_or(Value::Null);
        let id = projection_text(&incomplete, "id", "observability_control_board");
        let (next_repair, narrow_rerun) = observability_repair(id);
        return json!({
            "id": id,
            "surface": projection_text(&incomplete, "family", "observability"),
            "failure_class": "unobservable_authority_surface",
            "why_failed": format!("observability control board is {}", projection_text(board, "status", "missing")),
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
                "failure_class": projection_text(receipt, "failure_class", "stale_or_missing_source_local_authority"),
                "why_failed": format!(
                    "{} status={} current={}",
                    receipt.get("path").and_then(Value::as_str).unwrap_or("receipt"),
                    projection_text(receipt, "status", "unknown"),
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

pub(crate) fn observability_repair(id: &str) -> (String, String) {
    let narrow_rerun = format!("ultragoal observe command-roundtrip --command \"{id}\"");
    (
        format!(
            "adopt {id} in the successor observability catalog, then run {narrow_rerun} and inspect same-candidate query proof"
        ),
        narrow_rerun,
    )
}

pub(crate) fn receipt_repair(receipt: &Value) -> String {
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

pub(crate) fn receipt_rerun(receipt: &Value) -> String {
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

fn projection_text<'a>(value: &'a Value, field: &str, default: &'a str) -> &'a str {
    value.get(field).and_then(Value::as_str).unwrap_or(default)
}

pub(crate) fn git_status(root: &Path) -> Value {
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
