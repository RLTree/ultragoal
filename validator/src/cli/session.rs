use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const RECEIPT: &str = "validation_artifacts/harness/session-log-hardening-receipt.json";

#[derive(Debug)]
pub(crate) struct SessionCommand {
    pub(crate) receipt: PathBuf,
}

#[cfg(test)]
pub(crate) fn parse(raw: &[String]) -> Result<Option<SessionCommand>, String> {
    match raw {
        [a, b, c, ..] if a == "session-log" && b == "hardening" && c == "rebind" => {
            Ok(Some(SessionCommand {
                receipt: opt_path(raw, "--receipt")?,
            }))
        }
        [a, ..] if a == "session-log" => Err("unknown session-log command".into()),
        _ => Ok(None),
    }
}

pub(crate) fn run(root: &Path, command: &SessionCommand) -> Result<i32, String> {
    let value = rebind(root, &command.receipt)?;
    println!(
        "ultragoal-session-log-hardening {} receipt={}",
        value["status"],
        command.receipt.display()
    );
    Ok(0)
}

pub(crate) fn rebind(root: &Path, receipt: &Path) -> Result<Value, String> {
    validate_receipt_path(root, receipt)?;
    let claim_receipt_path =
        crate::output_path::claim_artifact_path(root, receipt, "session-log hardening receipt")?;
    let mut value = crate::json_boundary::read_json(&claim_receipt_path)?;
    value["generated_at"] = json!(crate::audit::clock::now_iso());
    value["candidate_version"] = json!(current_manifest_version(root)?);
    value["package_digest"] = json!(crate::package::inventory::package_digest(root)?);
    crate::json_boundary::write_json(&claim_receipt_path, &value)?;
    validate_rebound(root)?;
    Ok(value)
}

fn validate_rebound(root: &Path) -> Result<(), String> {
    let store = crate::schema_catalog::load(root);
    let failures = crate::audit::session_log_hardening::package_failures(root, &store);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "session-log hardening receipt did not validate: {}",
            failures.join(" | ")
        ))
    }
}

fn validate_receipt_path(root: &Path, receipt: &Path) -> Result<(), String> {
    let text = receipt.to_string_lossy();
    if text != RECEIPT {
        return Err(format!("session-log hardening receipt must be {RECEIPT}"));
    }
    if crate::package::inventory::package_path_error(root, &text).is_some() {
        return Err("session-log hardening receipt path escapes package root".into());
    }
    Ok(())
}

fn current_manifest_version(root: &Path) -> Result<String, String> {
    let manifest = crate::json_boundary::read_json(&root.join("plugin-manifest-draft.json"))?;
    let plugin = crate::json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))?;
    let manifest_version = string(&manifest, "version");
    let plugin_version = string(&plugin, "version");
    if manifest_version.is_empty() || plugin_version.is_empty() {
        return Err("missing manifest or plugin version".to_string());
    }
    if manifest_version != plugin_version {
        return Err(format!(
            "manifest version {manifest_version} != plugin version {plugin_version}"
        ));
    }
    Ok(manifest_version.to_string())
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

#[cfg(test)]
fn opt_path(args: &[String], key: &str) -> Result<PathBuf, String> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|index| args.get(index + 1))
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing required argument {key}"))
}
