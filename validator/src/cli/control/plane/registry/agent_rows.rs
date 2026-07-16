use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) fn for_home(
    root: &Path,
    home: Option<PathBuf>,
    candidate: &str,
    session_id: &str,
    package_root: Option<PathBuf>,
    project_root: Option<PathBuf>,
) -> Vec<Value> {
    let plugin = crate::cli::control::plane::surface::target::plugin_metadata(root);
    let home_root = crate::cli::control::plane::surface::target::home_root_from(root, home);
    let install_root = home_root.join(".codex/plugins/harness-ultragoal");
    let cache_root = home_root
        .join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal")
        .join(&plugin.version);
    let session_binding = crate::digest::bytes(session_id.as_bytes());
    let authority = crate::plugin_product::agent_discovery::observe_local_authority(
        crate::plugin_product::agent_discovery::LocalAgentAuthorityRequest {
            source_root: root,
            package_root: package_root.unwrap_or_else(|| root.to_path_buf()),
            installed_root: install_root,
            cache_root,
            global_root: home_root,
            project_root: project_root.unwrap_or_else(|| root.to_path_buf()),
            candidate_id: candidate,
            session_id: &session_binding,
        },
    );
    let failure_code = authority.as_ref().err().map(|error| error.id().code());
    crate::review::round::config::REVIEW_ROLES
        .iter()
        .map(|spec| {
            authority
                .as_ref()
                .ok()
                .and_then(|authority| {
                    authority
                        .roles()
                        .iter()
                        .find(|role| role.name() == spec.role_name)
                        .map(|role| verified_row(authority, role))
                })
                .unwrap_or_else(|| {
                    unavailable_row(spec.role_name, spec.agent_manifest_path, failure_code)
                })
        })
        .collect()
}

fn verified_row(
    authority: &crate::plugin_product::agent_discovery::LocalAgentAuthorityObservation,
    role: &crate::plugin_product::agent_discovery::LocalAgentRoleObservation,
) -> Value {
    json!({
        "role": role.name(),
        "agent_manifest_path": role.manifest_path(),
        "agent_manifest_digest": role.descriptor_sha256(),
        "source_manifest_present": true,
        "sandbox_mode": "read-only",
        "disk_cache_synced": role.package_matches()
            && role.installed_matches()
            && role.cache_matches(),
        "global_toml_present": role.global_matches(),
        "project_toml_present": role.project_matches(),
        "local_authority_status": "verified",
        "local_authority_scope": "source-layout-read-only",
        "local_authority_catalog_digest": authority.source_catalog_sha256(),
        "local_authority_binding_digest": authority.binding_sha256(),
        "runtime_metadata_status": "unavailable",
        "custom_agent_discovery_status": "unavailable",
        "exposed": false
    })
}

fn unavailable_row(role: &str, manifest_path: &str, failure_code: Option<&str>) -> Value {
    json!({
        "role": role,
        "agent_manifest_path": manifest_path,
        "agent_manifest_digest": "unavailable",
        "source_manifest_present": false,
        "sandbox_mode": "unavailable",
        "disk_cache_synced": false,
        "global_toml_present": false,
        "project_toml_present": false,
        "local_authority_status": "unavailable",
        "local_authority_scope": "source-layout-read-only",
        "local_authority_catalog_digest": "unavailable",
        "local_authority_binding_digest": "unavailable",
        "local_authority_failure_code": failure_code.unwrap_or("observation-unavailable"),
        "runtime_metadata_status": "unavailable",
        "custom_agent_discovery_status": "unavailable",
        "exposed": false
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn rows_use_four_current_manifests_and_withhold_runtime_discovery() {
        let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("agent-rows");
        let home = root.join("home");
        let package = root.join("package");
        let project = root.join("project");
        std::fs::create_dir_all(home.join(".codex/agents")).expect("empty global agent root");
        std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin root");
        let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repository root");
        std::fs::copy(
            repository_root.join(".codex-plugin/plugin.json"),
            root.join(".codex-plugin/plugin.json"),
        )
        .expect("plugin manifest");
        let version = crate::cli::control::plane::surface::target::plugin_metadata(&root).version;
        for spec in crate::agent_roles::CANONICAL_AGENT_ROLES {
            let bytes = format!(
                "name = \"{}\"\ndescription = \"review\"\ndeveloper_instructions = \"review\"\nsandbox_mode = \"read-only\"\n",
                spec.name
            );
            for path in [
                root.join(spec.manifest_path),
                package.join(spec.manifest_path),
                project.join(spec.manifest_path),
                home.join(".codex/plugins/harness-ultragoal")
                    .join(spec.manifest_path),
                home.join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal")
                    .join(&version)
                    .join(spec.manifest_path),
            ] {
                std::fs::create_dir_all(path.parent().unwrap()).expect("agent parent");
                std::fs::write(path, &bytes).expect("agent bytes");
            }
        }
        for destination in [
            package.join(".codex-plugin/plugin.json"),
            project.join(".codex-plugin/plugin.json"),
            home.join(".codex/plugins/harness-ultragoal/.codex-plugin/plugin.json"),
            home.join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal")
                .join(&version)
                .join(".codex-plugin/plugin.json"),
        ] {
            std::fs::create_dir_all(destination.parent().unwrap()).expect("plugin parent");
            std::fs::copy(root.join(".codex-plugin/plugin.json"), destination)
                .expect("plugin copy");
        }
        let candidate = crate::digest::bytes(b"agent-row-candidate");
        let before = super::for_home(
            &root,
            Some(home.clone()),
            &candidate,
            "session",
            Some(package.clone()),
            Some(project.clone()),
        );
        assert_eq!(before.len(), 4);
        assert!(before.iter().all(|row| {
            row["agent_manifest_path"]
                .as_str()
                .unwrap()
                .starts_with(".codex/agents/")
                && row.get("agent_type").is_none()
                && row["sandbox_mode"] == json!("read-only")
                && row["disk_cache_synced"] == json!(true)
                && row["local_authority_status"] == json!("verified")
                && row["runtime_metadata_status"] == json!("unavailable")
                && row["custom_agent_discovery_status"] == json!("unavailable")
                && row["exposed"] == json!(false)
        }));

        let legacy = root.join("custom-agents/harness-contract-claim-falsifier.toml");
        std::fs::create_dir_all(legacy.parent().unwrap()).expect("legacy parent");
        std::fs::write(legacy, "model_reasoning_effort = \"high\"\n").expect("legacy file");
        let after = super::for_home(
            &root,
            Some(home),
            &candidate,
            "session",
            Some(package),
            Some(project),
        );
        for (before_row, after_row) in before.iter().zip(&after) {
            assert_ne!(
                before_row["local_authority_binding_digest"],
                after_row["local_authority_binding_digest"]
            );
            let mut before_stable = before_row.clone();
            let mut after_stable = after_row.clone();
            before_stable
                .as_object_mut()
                .unwrap()
                .remove("local_authority_binding_digest");
            after_stable
                .as_object_mut()
                .unwrap()
                .remove("local_authority_binding_digest");
            assert_eq!(after_stable, before_stable);
        }
        std::fs::remove_dir_all(root).expect("cleanup agent rows");
    }
}
