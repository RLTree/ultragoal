use crate::audit::contract::{Failure, REQUIRED_AGENTS, REQUIRED_SKILLS};
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[cfg(test)]
pub fn check_plugin(bundle: &Value, root: &Path, out: &mut Vec<Failure>) {
    let manifest = &bundle["plugin_manifest"];
    check_plugin_manifest(manifest, root, out);
}

pub(crate) fn check_plugin_with_cache(
    bundle: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
    cache: &mut BTreeMap<String, Vec<Failure>>,
) {
    let manifest = &bundle["plugin_manifest"];
    let digest = crate::digest::canonical_json(manifest);
    if let Some(cached) = cache.get(&digest) {
        out.extend(cached.clone());
        return;
    }
    let start = out.len();
    check_plugin_manifest(manifest, root, out);
    cache.insert(digest, out[start..].to_vec());
}

fn check_plugin_manifest(manifest: &Value, root: &Path, out: &mut Vec<Failure>) {
    let plugin_paths = crate::package::inventory::inventory_paths(manifest);
    super::retired_reviewer_policy::check_packaged_paths(&plugin_paths, out);
    for failure in crate::package::resource::purpose::failures(root, manifest) {
        out.push(Failure::new(
            "plugin-inventory-closure",
            failure.code,
            failure.detail,
        ));
    }
    for path in &plugin_paths {
        if let Some(error) = crate::package::inventory::package_path_error(root, path) {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "plugin_path_invalid",
                format!("{path}: {error}"),
            ));
        } else if crate::package::inventory::resolve(root, path)
            .map(|p| !p.exists())
            .unwrap_or(true)
        {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "plugin_agent_path_missing",
                path,
            ));
        }
    }
    required_skill_checks(manifest, out);
    required_agent_checks(manifest, out);
    custom_agent_checks(manifest, root, out);
    for failure in crate::skill_links::manifest_failures(root, manifest) {
        out.push(Failure::new(
            "skill-inventory-closure",
            failure.code,
            failure.detail,
        ));
    }
}

fn required_skill_checks(manifest: &Value, out: &mut Vec<Failure>) {
    let skills = manifest
        .get("skills")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| str_field(row, "name"))
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_SKILLS {
        if !skills.contains(*required) {
            out.push(Failure::new(
                "skill-inventory-closure",
                "required_skill_missing",
                *required,
            ));
        }
    }
}

fn custom_agent_checks(manifest: &Value, root: &Path, out: &mut Vec<Failure>) {
    for entry in manifest
        .get("agents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let path = str_field(entry, "path");
        if !path.starts_with("custom-agents/") {
            continue;
        }
        let expected = str_field(entry, "app_visible_name");
        if expected.trim().is_empty() {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "custom_agent_app_visible_name_missing",
                path,
            ));
            continue;
        }
        let full = match crate::package::inventory::resolve(root, &path) {
            Ok(path) => path,
            Err(err) => {
                out.push(Failure::new(
                    "plugin-inventory-closure",
                    "custom_agent_toml_unreadable",
                    format!("{path}: {err}"),
                ));
                continue;
            }
        };
        let bytes = match crate::digest::read_file_bytes(&full) {
            Ok(bytes) => bytes,
            Err(err) => {
                out.push(Failure::new(
                    "plugin-inventory-closure",
                    "custom_agent_toml_unreadable",
                    format!("{path}: {err}"),
                ));
                continue;
            }
        };
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(err) => {
                out.push(Failure::new(
                    "plugin-inventory-closure",
                    "custom_agent_toml_unreadable",
                    format!("{path}: {err}"),
                ));
                continue;
            }
        };
        custom_agent_field_checks(&path, &text, &expected, out);
    }
}

fn custom_agent_field_checks(path: &str, text: &str, expected: &str, out: &mut Vec<Failure>) {
    for key in ["name", "description", "developer_instructions"] {
        if toml_string_field(text, key)
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "custom_agent_toml_field_missing",
                format!("{path}:{key}"),
            ));
        }
    }
    if toml_string_field(text, "name").unwrap_or_default() != expected {
        out.push(Failure::new(
            "plugin-inventory-closure",
            "custom_agent_name_mismatch",
            path,
        ));
    }
    reviewer_agent_runtime_checks(path, text, out);
}

fn reviewer_agent_runtime_checks(path: &str, text: &str, out: &mut Vec<Failure>) {
    if !crate::review::round::config::PERSONAS
        .iter()
        .any(|spec| spec.custom_path == path)
    {
        return;
    }
    for (key, want, code) in [
        (
            "model_reasoning_effort",
            "high",
            "custom_agent_reasoning_effort_not_high",
        ),
        (
            "sandbox_mode",
            "read-only",
            "custom_agent_sandbox_not_read_only",
        ),
    ] {
        if toml_string_field(text, key).unwrap_or_default() != want {
            out.push(Failure::new("plugin-inventory-closure", code, path));
        }
    }
}

fn toml_string_field(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} =");
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix(&prefix) else {
            continue;
        };
        let value = rest.trim();
        if value.starts_with("\"\"\"") {
            return Some("multiline-string".to_string());
        }
        if let Some(value) = value.strip_prefix('"').and_then(|s| s.split('"').next()) {
            return Some(value.to_string());
        }
    }
    None
}

fn required_agent_checks(manifest: &Value, out: &mut Vec<Failure>) {
    let agents = manifest
        .get("agents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| str_field(row, "name"))
        .collect::<BTreeSet<_>>();
    for required in REQUIRED_AGENTS {
        if !agents.contains(*required) {
            out.push(Failure::new(
                "plugin-inventory-closure",
                "plugin_agent_path_missing",
                *required,
            ));
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
