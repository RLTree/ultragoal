use crate::audit::AuditOptions;
use crate::audit::receipt;
use crate::json_boundary;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct ReceiptParts {
    pub options: AuditOptions,
    pub red_report: PathBuf,
    pub stdout: PathBuf,
    pub stderr: PathBuf,
    pub check_ids: Vec<String>,
    pub failures: BTreeMap<String, Vec<String>>,
    pub red: BTreeMap<String, Value>,
    pub target_artifacts: Vec<Value>,
    pub start: String,
    pub status: &'static str,
    pub validator_artifacts: Vec<Value>,
    pub scheduler_metrics: Vec<crate::scheduler::Metrics>,
}

pub fn package_status(failures: &BTreeMap<String, Vec<String>>) -> &'static str {
    if failures.values().any(|rows| !rows.is_empty()) {
        "fail"
    } else {
        "pass"
    }
}

pub(crate) fn red_report_status(red: &BTreeMap<String, Value>) -> &'static str {
    if red.is_empty()
        || red
            .values()
            .any(|row| row.get("status").and_then(Value::as_str) != Some("pass"))
    {
        "fail"
    } else {
        "pass"
    }
}

pub fn write_red_report(
    root: &Path,
    path: &Path,
    status: &str,
    red: &BTreeMap<String, Value>,
) -> Result<(), String> {
    let target_digest = crate::package::inventory::package_digest(root)?;
    json_boundary::write_json(
        path,
        &json!({
            "schema": "harness-ultragoal.red-fixture-report.v1",
            "status": status,
            "target_revision": {"kind": "package_digest", "value": target_digest},
            "red_fixtures": red,
            "generated_at": crate::audit::clock::now_iso()
        }),
    )
}

pub fn write_stdio_receipts(
    receipt: &Path,
    status: &str,
    red_len: usize,
) -> Result<(PathBuf, PathBuf), String> {
    let stdout = receipt.with_extension("stdout.txt");
    let stderr = receipt.with_extension("stderr.txt");
    crate::output_path::write(
        &stdout,
        format!("ultragoal-audit status={status} red={red_len}\n"),
        "stdout",
    )?;
    crate::output_path::write(&stderr, "", "stderr")?;
    Ok((stdout, stderr))
}

pub fn write_validator_receipt(parts: ReceiptParts) -> Result<i32, String> {
    let validator_receipt = receipt::build(receipt::ReceiptInput {
        root: parts.options.root.clone(),
        red_report: parts.red_report,
        stdout: parts.stdout,
        stderr: parts.stderr,
        check_ids: parts.check_ids,
        failures: parts.failures,
        red: parts.red,
        target_artifacts: parts.target_artifacts,
        start: parts.start,
        status: parts.status.to_string(),
        validator_artifacts: parts.validator_artifacts,
        command_text: parts.options.command_text,
        mode: parts.options.mode,
        scheduler_metrics: parts.scheduler_metrics,
    })?;
    json_boundary::write_json(&parts.options.receipt, &validator_receipt)?;
    println!(
        "ultragoal-audit {} receipt={}",
        parts.status,
        parts.options.receipt.display()
    );
    Ok(if parts.status == "pass" { 0 } else { 1 })
}
