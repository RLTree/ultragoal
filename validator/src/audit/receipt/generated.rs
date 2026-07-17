use super::ReceiptInput;
use crate::digest;
use serde_json::{Value, json};

pub(super) fn artifacts(input: &ReceiptInput, run_id: &str) -> Result<Vec<Value>, String> {
    let out = vec![json!({
        "artifact_type": "red_fixture_report",
        "path": super::rel_path(&input.root, &input.red_report),
        "digest": digest::file(&input.red_report)?,
        "validator_run_id": run_id,
        "input_digest": digest::file(&input.root.join("templates/RED_FIXTURES.json"))?,
        "generated_at": crate::audit::clock::now_iso()
    })];
    Ok(out)
}
