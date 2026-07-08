use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
#[path = "receipt_fields.rs"]
mod receipt_fields;
use super::executor::CoverageExecution;

const ROUTINE_REPORT_REL: &str = "validation_artifacts/coverage/llvm-cov-routine.json";
const ROUTINE_TARGET_DIR: &str = "target/ultragoal-routine-coverage";
const ROUTINE_BLOCKED: &[&str] = &[
    "complete_coverage",
    "completion",
    "package_readiness",
    "review_readiness",
    "release_readiness",
    "final_packet_correctness",
    "update_goal_eligibility",
    "app_registry_or_reviewer_exposure",
];

pub(super) fn execute(root: &Path, receipt: &Path, candidate: &str) -> CoverageExecution {
    match execute_inner(root, receipt, candidate) {
        Ok(()) => CoverageExecution {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
            cache_mode: "coverage_routine_verified_local",
        },
        Err(err) => CoverageExecution {
            code: 2,
            stdout: String::new(),
            stderr: format!("coverage_routine_observation_failed:{err}"),
            cache_mode: "coverage_routine_verified_local",
        },
    }
}

fn execute_inner(root: &Path, receipt: &Path, candidate: &str) -> Result<(), String> {
    let report = crate::output_path::literal_claim_artifact_path(
        root,
        ROUTINE_REPORT_REL,
        "routine coverage report",
    );
    let output = Command::new("cargo")
        .arg("llvm-cov")
        .arg("--workspace")
        .arg("--all-features")
        .arg("--json")
        .arg("--summary-only")
        .arg("--output-path")
        .arg(&report)
        .arg("--offline")
        .arg("--no-clean")
        .arg("--")
        .arg("coverage")
        .current_dir(root)
        .env("CARGO_TARGET_DIR", ROUTINE_TARGET_DIR)
        .output()
        .map_err(|err| format!("spawn cargo llvm-cov routine: {err}"))?;
    if !output.status.success() {
        return Err(first_process_line(&output.stderr, &output.stdout));
    }
    write_receipt(root, receipt, candidate, &report)
}

fn write_receipt(
    root: &Path,
    receipt: &Path,
    candidate: &str,
    report: &Path,
) -> Result<(), String> {
    let manifest_path = receipt_fields::coverage_manifest_path(root);
    let command_path = receipt_fields::coverage_command_path(root);
    let manifest = crate::json_boundary::read_json(&manifest_path)?;
    let report_value = crate::json_boundary::read_json(report)?;
    let source_tree_digest =
        crate::claim_semantics::coverage::digests::source_tree_digest(root, &manifest)?;
    let manifest_digest = crate::digest::file(&manifest_path)?;
    let command_digest = crate::digest::file(&command_path)?;
    let uncovered_records = uncovered_records(root, &report_value);
    let dimensions = measured_dimensions(&manifest);
    let receipt_value = serde_json::json!({
        "schema": "harness-ultragoal.coverage-receipt.v1",
        "claim_id": "CLAIM-001",
        "command": "cargo llvm-cov --workspace --all-features --json --summary-only --offline --no-clean -- coverage",
        "tool": "cargo-llvm-cov",
        "tool_version": tool_version(),
        "cargo_version": command_version("cargo", &["--version"]),
        "rustc_version": command_version("rustc", &["--version"]),
        "flags": ["--json", "--summary-only", "--offline", "--no-clean", "--", "coverage"],
        "coverage_target_dir": ROUTINE_TARGET_DIR,
        "coverage_cache_class": "retained_artifact_verified_local",
        "source_tree_digest": source_tree_digest,
        "coverage_manifest_digest": manifest_digest,
        "coverage_command_digest": command_digest,
        "boundary_lineage_digest": lineage_digest(&source_tree_digest, &manifest_digest, &command_digest),
        "equivalence_status": "verified_current_input_equivalent",
        "current_candidate_digest": candidate,
        "target_revision": {"kind": "package_digest", "value": candidate},
        "command_exit": 0,
        "machine_readable_report": {"path": ROUTINE_REPORT_REL, "digest": crate::digest::file(report)?},
        "generated_by": "coverage-command",
        "percent_source": "machine_readable_report",
        "target_paths": receipt_fields::array_strings(&manifest, "required_target_paths"),
        "measured_dimensions": receipt_fields::sorted(dimensions),
        "coverage": {"percent": coverage_percent(&report_value), "floor_percent": 0, "policy": "routine_repair_feedback"},
        "uncovered_count": uncovered_records.len(),
        "uncovered_records": uncovered_records,
        "exclusions": manifest.get("exclusions").cloned().unwrap_or_else(|| serde_json::json!([])),
        "claim_ceiling": "routine_repair_only",
        "supported_claim_classes": ["routine_coverage_feedback"],
        "blocked_claim_classes": ROUTINE_BLOCKED
    });
    let resolved = receipt_fields::resolve_receipt(root, receipt)?;
    crate::json_boundary::write_json(&resolved, &receipt_value)
}

fn measured_dimensions(manifest: &Value) -> Vec<String> {
    manifest
        .get("required_measured_dimensions_per_root")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|row| receipt_fields::array_strings(row, "dimensions"))
        .collect()
}

fn coverage_percent(report: &Value) -> f64 {
    report
        .pointer("/data/0/totals/lines/percent")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn lineage_digest(source_tree: &str, manifest: &str, command: &str) -> String {
    crate::digest::canonical_json(&serde_json::json!({
        "mode": "routine_repair_only",
        "source_tree_digest": source_tree,
        "coverage_manifest_digest": manifest,
        "coverage_command_digest": command
    }))
}

fn uncovered_records(root: &Path, report: &Value) -> Vec<Value> {
    report
        .pointer("/data/0/files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| uncovered_record(root, item))
        .collect()
}

fn uncovered_record(root: &Path, item: &Value) -> Option<Value> {
    let percent = item
        .pointer("/summary/lines/percent")
        .and_then(Value::as_f64)
        .unwrap_or(100.0);
    (percent < 100.0).then(|| {
        serde_json::json!({
            "path": report_path_label(root, &receipt_fields::string(item, "filename")),
            "reason": format!("routine line coverage {percent:.2}%")
        })
    })
}

fn report_path_label(root: &Path, filename: &str) -> String {
    let path = PathBuf::from(filename);
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let resolved = if path.is_absolute() {
        path
    } else {
        canonical_root.join(path)
    };
    resolved
        .strip_prefix(&canonical_root)
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|_| filename.to_string())
}

fn first_process_line(stderr: &[u8], stdout: &[u8]) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .chain(String::from_utf8_lossy(stdout).lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("cargo llvm-cov routine failed")
        .to_string()
}

fn tool_version() -> String {
    command_version("cargo", &["llvm-cov", "--version"])
}

fn command_version(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|version| !version.is_empty())
        .unwrap_or_else(|| format!("{program} version unavailable"))
}
