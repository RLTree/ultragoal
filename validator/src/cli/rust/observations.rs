use crate::cli::rust::types::RustOperation;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

pub(crate) struct ObservationSet {
    pub(crate) value: Value,
    pub(crate) failures: Vec<String>,
}

pub(crate) fn collect(root: &Path, operation: RustOperation) -> ObservationSet {
    collect_with_policy(
        root,
        operation,
        required_probes(operation),
        std::env::var("RUSTC_WRAPPER").is_ok(),
    )
}

#[cfg(test)]
pub(crate) fn collect_from_probes(
    root: &Path,
    operation: RustOperation,
    probes: Vec<Probe>,
) -> ObservationSet {
    collect_with_policy(
        root,
        operation,
        probes,
        std::env::var("RUSTC_WRAPPER").is_ok(),
    )
}

pub(crate) fn collect_with_policy(
    root: &Path,
    operation: RustOperation,
    probes: Vec<Probe>,
    rustc_wrapper_present: bool,
) -> ObservationSet {
    let mut failures = Vec::new();
    let probes = probes
        .into_iter()
        .map(|probe| {
            let observed = command(probe.program, probe.args);
            if !observed
                .get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                failures.push(format!("rust_devx_required_tool_probe_failed:{}", probe.id));
            }
            json!({
                "id": probe.id,
                "required": true,
                "program": probe.program,
                "args": probe.args,
                "observation": observed
            })
        })
        .collect::<Vec<_>>();
    failures.extend(policy_failures(root, operation, rustc_wrapper_present));
    ObservationSet {
        value: json!({
            "probes": probes,
            "raw_output_is_authority": false,
            "missing_tool_is_claim_blocking": true,
            "operation": operation.id()
        }),
        failures,
    }
}

pub(crate) fn command(program: &str, args: &[&str]) -> Value {
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            json!({
                "available": true,
                "success": output.status.success(),
                "status_code": output.status.code(),
                "stdout_digest": crate::digest::bytes(stdout.as_bytes()),
                "stderr_digest": crate::digest::bytes(stderr.as_bytes()),
                "stdout_excerpt": excerpt(&redact_private_paths(&stdout)),
                "stderr_excerpt": excerpt(&redact_private_paths(&stderr))
            })
        }
        Err(err) => json!({
            "available": false,
            "success": false,
            "error": err.to_string()
        }),
    }
}

fn policy_failures(
    root: &Path,
    operation: RustOperation,
    rustc_wrapper_present: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    for rel in [
        "rust-toolchain.toml",
        "Cargo.lock",
        "Cargo.toml",
        ".cargo/config.toml",
    ] {
        if !root.join(rel).is_file() {
            out.push(format!("rust_devx_required_substrate_missing:{rel}"));
        }
    }
    if operation == RustOperation::CleanProof && rustc_wrapper_present {
        out.push("rust_devx_clean_proof_hidden_rustc_wrapper".to_string());
    }
    out
}

fn required_probes(operation: RustOperation) -> Vec<Probe> {
    let mut probes = vec![
        Probe::new("rustc-version", "rustc", &["--version", "--verbose"]),
        Probe::new("cargo-version", "cargo", &["--version", "--verbose"]),
        Probe::new("active-toolchain", "rustup", &["show", "active-toolchain"]),
        Probe::new(
            "cargo-metadata-locked",
            "cargo",
            &["metadata", "--format-version=1", "--locked", "--no-deps"],
        ),
    ];
    match operation {
        RustOperation::Standard | RustOperation::Release => {
            probes.push(Probe::new(
                "cargo-nextest-version",
                "cargo",
                &["nextest", "--version"],
            ));
            probes.push(Probe::new(
                "cargo-clippy-version",
                "cargo",
                &["clippy", "--version"],
            ));
        }
        RustOperation::DependencyAudit => {
            probes.push(Probe::new(
                "cargo-deny-version",
                "cargo",
                &["deny", "--version"],
            ));
            probes.push(Probe::new(
                "cargo-audit-version",
                "cargo",
                &["audit", "--version"],
            ));
        }
        RustOperation::CoverageProve => {
            probes.push(Probe::new(
                "cargo-llvm-cov-version",
                "cargo",
                &["llvm-cov", "--version"],
            ));
        }
        RustOperation::Watch => {
            probes.push(Probe::new("watchexec-version", "watchexec", &["--version"]));
        }
        _ => {}
    }
    probes
}

pub(crate) struct Probe {
    id: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

impl Probe {
    pub(crate) fn new(
        id: &'static str,
        program: &'static str,
        args: &'static [&'static str],
    ) -> Self {
        Self { id, program, args }
    }
}

fn excerpt(text: &str) -> String {
    text.chars().take(500).collect()
}

pub(crate) fn redact_private_paths(text: &str) -> String {
    let mut redacted = text.to_string();
    if let Some(home) = std::env::var_os("HOME").and_then(|value| value.into_string().ok()) {
        redacted = redacted.replace(&home, "[redacted-home-path]");
    }
    redact_users_paths(&redacted)
}

fn redact_users_paths(text: &str) -> String {
    let marker = users_marker();
    text.split_whitespace()
        .map(|token| {
            if token.contains(marker) {
                redact_users_path_token(token)
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_users_path_token(token: &str) -> String {
    let marker = users_marker();
    let mut out = String::new();
    let mut rest = token;
    while let Some(index) = rest.find(marker) {
        out.push_str(&rest[..index]);
        let path_tail = &rest[index..];
        let end = path_tail
            .find(|ch: char| {
                matches!(
                    ch,
                    '"' | '\'' | ')' | ']' | '}' | ',' | ';' | ':' | '\n' | '\r' | '\t'
                )
            })
            .unwrap_or(path_tail.len());
        out.push_str("[redacted-home-path]");
        rest = &path_tail[end..];
    }
    out.push_str(rest);
    out
}

fn users_marker() -> &'static str {
    concat!("/", "Users/")
}

#[cfg(test)]
pub(crate) fn redacted_excerpt_for_test(text: &str) -> String {
    excerpt(&redact_private_paths(text))
}
