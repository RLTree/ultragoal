use super::ReceiptInput;
use crate::digest;
use serde_json::{Value, json};
use std::path::PathBuf;

pub(super) fn artifacts(input: &ReceiptInput, run_id: &str) -> Result<Vec<Value>, String> {
    let mut out = vec![json!({
        "artifact_type": "red_fixture_report",
        "path": super::rel_path(&input.root, &input.red_report),
        "digest": digest::file(&input.red_report)?,
        "validator_run_id": run_id,
        "input_digest": digest::file(&input.root.join("templates/RED_FIXTURES.json"))?,
        "generated_at": crate::audit::clock::now_iso()
    })];
    for row in &input.target_artifacts {
        let mut item = row.clone();
        item["validator_run_id"] = json!(run_id);
        if let Some(path) = item.get("path").and_then(Value::as_str) {
            item["path"] = json!(super::rel_path(&input.root, &PathBuf::from(path)));
        }
        out.push(item);
    }
    out.extend(ready_for_merge(input, run_id)?);
    Ok(out)
}

fn ready_for_merge(input: &ReceiptInput, run_id: &str) -> Result<Vec<Value>, String> {
    let dir = input.root.join("examples/generated");
    let mut out = Vec::new();
    for entry in super::generated_dir_entries(std::fs::read_dir(&dir))? {
        let path = super::generated_entry_path(entry)?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !name.starts_with("READY_FOR_MERGE") || !name.ends_with(".json") {
            continue;
        }
        out.push(json!({
            "artifact_type": "ready_for_merge",
            "path": super::rel_path(&input.root, &path),
            "digest": digest::file(&path)?,
            "validator_run_id": run_id,
            "input_digest": digest::file(&input.root.join("fixtures/valid/minimal-goal-run.json"))?,
            "generated_at": crate::audit::clock::now_iso()
        }));
    }
    out.sort_by(|a, b| {
        a.get("path")
            .and_then(Value::as_str)
            .cmp(&b.get("path").and_then(Value::as_str))
    });
    Ok(out)
}
