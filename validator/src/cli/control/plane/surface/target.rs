use crate::cli::control::plane::types::ControlOperation;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct PluginMetadata {
    pub(crate) name: String,
    pub(crate) version: String,
}

pub(crate) fn plugin_metadata(root: &Path) -> PluginMetadata {
    let value = crate::json_boundary::read_json(&root.join(".codex-plugin/plugin.json"))
        .unwrap_or_else(|_| json!({}));
    PluginMetadata {
        name: string_or_unknown(&value, "name"),
        version: string_or_unknown(&value, "version"),
    }
}

pub(crate) fn source_value(root: &Path, plugin: &PluginMetadata) -> Value {
    json!({
        "package_digest": crate::package::inventory::package_digest(root).unwrap_or_default(),
        "plugin_manifest_digest": crate::digest::file(&root.join(".codex-plugin/plugin.json")).ok(),
        "plugin_name": plugin.name,
        "plugin_version": plugin.version
    })
}

pub(crate) fn state(
    target_root: &Path,
    operation: ControlOperation,
    expected: &str,
    source_plugin: &PluginMetadata,
) -> Value {
    let target_plugin = plugin_metadata(target_root);
    let digest = crate::package::inventory::package_digest(target_root);
    json!({
        "surface": surface_id(operation),
        "logical_path": logical_path(operation, source_plugin),
        "path_authority": path_authority(target_root),
        "exists": target_root.join("plugin-manifest-draft.json").is_file(),
        "package_digest": digest.as_ref().ok(),
        "package_error": digest.as_ref().err(),
        "plugin_manifest_digest": crate::digest::file(&target_root.join(".codex-plugin/plugin.json")).ok(),
        "plugin_name": target_plugin.name,
        "plugin_version": target_plugin.version,
        "expected_package_digest": expected,
        "source_plugin_version": source_plugin.version
    })
}

pub(crate) fn default_root(root: &Path, operation: ControlOperation, version: &str) -> PathBuf {
    let home = home_root_from(root, std::env::var_os("HOME").map(PathBuf::from));
    match operation {
        ControlOperation::InstallAudit => home.join(".codex/plugins/harness-ultragoal"),
        ControlOperation::CacheAudit => home
            .join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal")
            .join(version),
        _ => root.to_path_buf(),
    }
}

pub(crate) fn home_root_from(root: &Path, home: Option<PathBuf>) -> PathBuf {
    home.unwrap_or_else(|| root.to_path_buf())
}

pub(crate) fn surface_id(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::InstallAudit => "installed_plugin",
        ControlOperation::CacheAudit => "versioned_cache_package",
        _ => "unsupported_surface",
    }
}

fn logical_path(operation: ControlOperation, plugin: &PluginMetadata) -> String {
    match operation {
        ControlOperation::InstallAudit => format!("codex-plugin-install-{}", plugin.name),
        ControlOperation::CacheAudit => {
            format!(
                "codex-plugin-cache-{}-{}",
                plugin.name,
                kebab_segment(&plugin.version)
            )
        }
        _ => "unsupported-surface".to_string(),
    }
}

fn kebab_segment(value: &str) -> String {
    let mut out = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}

fn path_authority(target_root: &Path) -> &'static str {
    if target_root.is_absolute() {
        "cli_or_default_local_root_redacted"
    } else {
        "relative_test_root"
    }
}

fn string_or_unknown(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string()
}
