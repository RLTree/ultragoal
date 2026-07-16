use crate::self_tests::boundaries::workspace_fixtures;
use serde_json::json;
use std::path::Path;

#[test]
fn fail_closed_rows_report_local_authority_without_runtime_promotion() {
    let root = workspace_fixtures::temp_root("cli-registry-local-truth");
    let home = root.join("home");
    let package = root.join("package");
    let project = root.join("project");
    std::fs::create_dir_all(home.join(".codex/agents")).expect("empty global agent root");
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin root");
    std::fs::copy(
        workspace_fixtures::repo_root().join(".codex-plugin/plugin.json"),
        root.join(".codex-plugin/plugin.json"),
    )
    .expect("plugin manifest");
    let version = crate::cli::control::plane::surface::target::plugin_metadata(&root).version;
    for role in crate::agent_roles::CANONICAL_AGENT_ROLES {
        let source = root.join(role.manifest_path);
        let packaged = package.join(role.manifest_path);
        let projected = project.join(role.manifest_path);
        let installed = home
            .join(".codex/plugins/harness-ultragoal")
            .join(role.manifest_path);
        let cache = home
            .join(".codex/plugins/cache/local-harness-plugins/harness-ultragoal")
            .join(&version)
            .join(role.manifest_path);
        let bytes = format!(
            "name = \"{}\"\ndescription = \"Review.\"\ndeveloper_instructions = \"Review only.\"\nsandbox_mode = \"read-only\"\n",
            role.name
        );
        for path in [source, packaged, projected, installed, cache] {
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(path, &bytes).expect("write");
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
        std::fs::create_dir_all(destination.parent().expect("plugin parent"))
            .expect("plugin parent");
        std::fs::copy(root.join(".codex-plugin/plugin.json"), destination).expect("plugin copy");
    }

    let before = recursive_snapshot(&root);
    let rows = crate::cli::control::plane::registry::agent_types_for_home(
        &root,
        Some(home.clone()),
        &workspace_fixtures::sha('c'),
        "registry-local-session",
        Some(package),
        Some(project),
    );
    assert_eq!(rows.len(), 4);
    assert!(
        rows.iter().all(|row| {
            row.get("agent_type").is_none()
                && row["agent_manifest_path"]
                    .as_str()
                    .is_some_and(|path| path.starts_with(".codex/agents/"))
                && row["sandbox_mode"] == json!("read-only")
                && row["disk_cache_synced"] == json!(true)
                && row["global_toml_present"] == json!(false)
                && row["local_authority_status"] == json!("verified")
                && row["runtime_metadata_status"] == json!("unavailable")
                && row["custom_agent_discovery_status"] == json!("unavailable")
                && row["exposed"] == json!(false)
        }),
        "{rows:#?}"
    );
    assert_agent_rows_schema(&rows);
    let mut forged_verified = rows[0].clone();
    forged_verified
        .as_object_mut()
        .expect("verified agent row")
        .insert(
            "local_authority_failure_code".to_owned(),
            json!("invalid-binding"),
        );
    assert!(!agent_row_schema_errors(&forged_verified).is_empty());
    let mut unavailable_shape_with_verified_status = rows[0].clone();
    unavailable_shape_with_verified_status["agent_manifest_digest"] = json!("unavailable");
    unavailable_shape_with_verified_status["source_manifest_present"] = json!(false);
    unavailable_shape_with_verified_status["sandbox_mode"] = json!("unavailable");
    unavailable_shape_with_verified_status["disk_cache_synced"] = json!(false);
    unavailable_shape_with_verified_status["project_toml_present"] = json!(false);
    assert!(!agent_row_schema_errors(&unavailable_shape_with_verified_status).is_empty());
    assert_eq!(recursive_snapshot(&root), before);

    let write_capable = home.join(".codex/agents/unrelated-observer.toml");
    std::fs::write(
        &write_capable,
        "name = \"unrelated-observer\"\ndescription = \"Observer.\"\ndeveloper_instructions = \"Observe.\"\nsandbox_mode = \"workspace-write\"\n",
    )
    .expect("write-capable global fixture");
    let before_rejection = recursive_snapshot(&root);
    let rejected = crate::cli::control::plane::registry::agent_types_for_home(
        &root,
        Some(root.join("home")),
        &workspace_fixtures::sha('c'),
        "registry-local-session",
        Some(root.join("package")),
        Some(root.join("project")),
    );
    assert!(rejected.iter().all(|row| {
        row["local_authority_status"] == json!("unavailable")
            && row["local_authority_failure_code"] == json!("sandbox-policy-rejected")
            && row["runtime_metadata_status"] == json!("unavailable")
            && row["custom_agent_discovery_status"] == json!("unavailable")
            && row["exposed"] == json!(false)
    }));
    assert_agent_rows_schema(&rejected);
    let mut forged_unavailable = rejected[0].clone();
    forged_unavailable["local_authority_binding_digest"] = json!(workspace_fixtures::sha('d'));
    assert!(!agent_row_schema_errors(&forged_unavailable).is_empty());
    let mut verified_shape_with_unavailable_status = rejected[0].clone();
    verified_shape_with_unavailable_status["agent_manifest_digest"] =
        json!(workspace_fixtures::sha('d'));
    verified_shape_with_unavailable_status["source_manifest_present"] = json!(true);
    verified_shape_with_unavailable_status["sandbox_mode"] = json!("read-only");
    verified_shape_with_unavailable_status["disk_cache_synced"] = json!(true);
    verified_shape_with_unavailable_status["project_toml_present"] = json!(true);
    assert!(!agent_row_schema_errors(&verified_shape_with_unavailable_status).is_empty());
    assert_eq!(recursive_snapshot(&root), before_rejection);
    std::fs::remove_dir_all(root).expect("cleanup cli registry local truth");
}

fn assert_agent_rows_schema(rows: &[serde_json::Value]) {
    for row in rows {
        let errors = agent_row_schema_errors(row);
        assert!(errors.is_empty(), "{errors:#?}\n{row:#?}");
    }
}

fn agent_row_schema_errors(row: &serde_json::Value) -> Vec<String> {
    let repo = workspace_fixtures::repo_root();
    let schema =
        crate::json_boundary::read_json(&repo.join("schemas/codex-registry-exposure.schema.json"))
            .expect("registry schema");
    let mut row_schema = schema["$defs"]["agentTypeExposure"].clone();
    row_schema
        .as_object_mut()
        .expect("agent row schema")
        .insert(
            "$id".to_owned(),
            json!("https://harness-ultragoal.local/schemas/agent-type-exposure-test.json"),
        );
    crate::schema_catalog::bound_schema_errors("agent-type-exposure", &row_schema, row)
}

fn recursive_snapshot(root: &Path) -> Vec<(std::path::PathBuf, bool, Vec<u8>)> {
    let mut rows = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .map(|entry| {
            let entry = entry.expect("snapshot entry");
            let relative = entry.path().strip_prefix(root).expect("relative path");
            let is_file = entry.file_type().is_file();
            let bytes = if is_file {
                std::fs::read(entry.path()).expect("snapshot file")
            } else {
                Vec::new()
            };
            (relative.to_path_buf(), is_file, bytes)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}
