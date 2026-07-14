use crate::cli::control::plane::operation::ControlOperation;
use crate::cli::control::plane::surface;
use serde_json::json;
use std::path::Path;

fn write_json(path: &Path, value: &serde_json::Value) {
    std::fs::create_dir_all(path.parent().expect("json parent")).expect("json parent");
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_package(root: &Path) {
    std::fs::create_dir_all(root.join(".codex-plugin")).expect("plugin dir");
    std::fs::create_dir_all(root.join("docs")).expect("docs dir");
    write_json(
        &root.join(".codex-plugin/plugin.json"),
        &json!({"name":"harness-ultragoal","version":"0.0.test"}),
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[".codex-plugin/plugin.json","docs/a.txt"]}),
    );
    std::fs::write(root.join("docs/a.txt"), "same").expect("content");
}

#[test]
fn package_surface_target_defaults_cover_unavailable_targets() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("surface-target-contracts");
    write_package(&root);
    let plugin = surface::target::plugin_metadata(&root);
    let install_root =
        surface::target::default_root(&root, ControlOperation::InstallAudit, "0.0.test");
    let cache_root = surface::target::default_root(&root, ControlOperation::CacheAudit, "0.0.test");
    let fallback_root =
        surface::target::default_root(&root, ControlOperation::RegistryProbe, "0.0.test");
    let missing_home = surface::target::home_root_from(&root, None);
    assert!(install_root.ends_with(".codex/plugins/harness-ultragoal"));
    assert!(cache_root.ends_with("0.0.test"));
    assert_eq!(fallback_root, root);
    assert_eq!(missing_home, root);
    assert_eq!(
        surface::target::surface_id(ControlOperation::RegistryProbe),
        "unsupported_surface"
    );

    let missing_state = surface::target::state(
        Path::new("relative-missing-surface"),
        ControlOperation::RegistryProbe,
        &crate::self_tests::boundaries::workspace_fixtures::sha('d'),
        &plugin,
    );
    assert_eq!(missing_state["surface"], "unsupported_surface");
    assert_eq!(missing_state["logical_path"], "unsupported-surface");
    assert_eq!(missing_state["path_authority"], "relative_test_root");
    assert_eq!(missing_state["plugin_name"], "unknown");

    let cache_state = surface::target::state(
        Path::new("relative-missing-surface"),
        ControlOperation::CacheAudit,
        &crate::self_tests::boundaries::workspace_fixtures::sha('d'),
        &surface::target::PluginMetadata {
            name: "harness-ultragoal".to_string(),
            version: "0..test".to_string(),
        },
    );
    assert_eq!(
        cache_state["logical_path"],
        "codex-plugin-cache-harness-ultragoal-0-test"
    );

    let failures = surface::value_failures_for_target(
        ControlOperation::InstallAudit,
        &crate::self_tests::boundaries::workspace_fixtures::sha('e'),
        &missing_state,
    );
    for expected in [
        "package_surface_target_missing",
        "package_surface_wrong_surface",
        "package_surface_digest_mismatch",
        "package_surface_plugin_name_mismatch",
        "package_surface_plugin_version_mismatch",
        "package_surface_digest_unavailable",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup target contracts");
}
