use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) fn for_home(root: &Path, home: Option<PathBuf>) -> Vec<Value> {
    let plugin = crate::cli::control::plane::surface::target::plugin_metadata(root);
    let home_root = crate::cli::control::plane::surface::target::home_root_from(root, home);
    let install_root = home_root.join(".codex/plugins/harness-ultragoal");
    let cache_root = home_root
        .join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal")
        .join(&plugin.version);
    let global_agent_root = home_root.join(".codex/agents");
    crate::review::round::config::REVIEW_ROLES
        .iter()
        .map(|spec| {
            let role_name = spec.role_name;
            let manifest_path = spec.agent_manifest_path;
            let source = root.join(manifest_path);
            let source_digest = crate::digest::file(&source).ok();
            let disk_cache_synced = same_file_digest(&source, &install_root.join(manifest_path))
                && same_file_digest(&source, &cache_root.join(manifest_path));
            let global_toml_present = Path::new(manifest_path)
                .file_name()
                .is_some_and(|name| same_file_digest(&source, &global_agent_root.join(name)));
            json!({
                "role": role_name,
                "agent_manifest_path": manifest_path,
                "agent_manifest_digest": source_digest.as_deref().unwrap_or("unavailable"),
                "source_manifest_present": source_digest.is_some(),
                "sandbox_mode": sandbox_mode(&source).as_deref().unwrap_or("unavailable"),
                "disk_cache_synced": disk_cache_synced,
                "global_toml_present": global_toml_present,
                "runtime_metadata_status": "unavailable",
                "custom_agent_discovery_status": "unavailable",
                "exposed": false
            })
        })
        .collect()
}

fn sandbox_mode(path: &Path) -> Option<String> {
    let bytes = crate::digest::read_file_bytes(path).ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    let value = toml::from_str::<toml::Value>(text).ok()?;
    value.get("sandbox_mode")?.as_str().map(str::to_owned)
}

fn same_file_digest(left: &Path, right: &Path) -> bool {
    match (crate::digest::file(left), crate::digest::file(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn rows_use_four_current_manifests_and_withhold_runtime_discovery() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("agent-rows");
        let home = root.join("home");
        std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin root");
        std::fs::write(
            root.join(".codex-plugin/plugin.json"),
            serde_json::to_vec(&json!({"name":"harness-ultragoal","version":"0.0.0-test"}))
                .unwrap(),
        )
        .expect("plugin manifest");
        for spec in crate::review::round::config::REVIEW_ROLES {
            let bytes = format!(
                "name = \"{}\"\ndescription = \"review\"\ndeveloper_instructions = \"review\"\nsandbox_mode = \"read-only\"\n",
                spec.role_name
            );
            for path in [
                root.join(spec.agent_manifest_path),
                home.join(".codex/plugins/harness-ultragoal")
                    .join(spec.agent_manifest_path),
                home.join(
                    ".codex/plugins/cache/local-harness-plugins/harness-ultragoal/0.0.0-test",
                )
                .join(spec.agent_manifest_path),
                home.join(".codex/agents")
                    .join(Path::new(spec.agent_manifest_path).file_name().unwrap()),
            ] {
                std::fs::create_dir_all(path.parent().unwrap()).expect("agent parent");
                std::fs::write(path, &bytes).expect("agent bytes");
            }
        }
        let before = super::for_home(&root, Some(home.clone()));
        assert_eq!(before.len(), 4);
        assert!(before.iter().all(|row| {
            row["agent_manifest_path"]
                .as_str()
                .unwrap()
                .starts_with(".codex/agents/")
                && row.get("agent_type").is_none()
                && row["sandbox_mode"] == json!("read-only")
                && row["disk_cache_synced"] == json!(true)
                && row["runtime_metadata_status"] == json!("unavailable")
                && row["custom_agent_discovery_status"] == json!("unavailable")
                && row["exposed"] == json!(false)
        }));

        let legacy = root.join("custom-agents/harness-contract-claim-falsifier.toml");
        std::fs::create_dir_all(legacy.parent().unwrap()).expect("legacy parent");
        std::fs::write(legacy, "model_reasoning_effort = \"high\"\n").expect("legacy file");
        assert_eq!(super::for_home(&root, Some(home)), before);
        std::fs::remove_dir_all(root).expect("cleanup agent rows");
    }
}
