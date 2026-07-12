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
        if row.get("artifact_type").and_then(Value::as_str) == Some("ready_for_merge") {
            return Err(
                "reserved ready_for_merge artifact requires the typed HCT-CLAIMS channel"
                    .to_owned(),
            );
        }
        let mut item = row.clone();
        item["validator_run_id"] = json!(run_id);
        if let Some(path) = item.get("path").and_then(Value::as_str) {
            item["path"] = json!(super::rel_path(&input.root, &PathBuf::from(path)));
        }
        out.push(item);
    }
    Ok(out)
}
