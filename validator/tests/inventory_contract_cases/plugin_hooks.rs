use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, AuthorityCatalog, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request};

fn catalog(repo: &TestRepo) -> AuthorityCatalog {
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    InventoryBuilder::new(&context).build().unwrap()
}

fn has_code(catalog: &AuthorityCatalog, code: &str) -> bool {
    catalog
        .findings()
        .iter()
        .any(|finding| finding.code == code)
}

fn manifest(repo: &TestRepo) {
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture"}"#,
    );
}

#[test]
fn default_hook_files_are_validated_cataloged_and_never_imply_trust() {
    let repo = TestRepo::new("plugin-default-hook-valid");
    manifest(&repo);
    repo.write(
        "hooks/hooks.json",
        br#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"true"}]}]}}"#,
    );
    repo.commit();
    let valid = catalog(&repo);
    assert!(!has_code(&valid, "invalid_plugin_hooks"));
    assert!(has_code(&valid, "plugin_hook_trust_required"));
    assert!(valid.entries().iter().any(|entry| {
        entry.stable_id == "PLUGIN-HOOKS:hooks/hooks.json"
            && entry.active_status == ActiveStatus::Candidate
            && entry
                .input_provenance
                .contains(&"hook-source:host-default".to_owned())
    }));

    let repo = TestRepo::new("plugin-default-hook-malformed");
    manifest(&repo);
    repo.write(
        "hooks/hooks.json",
        br#"{"hooks":{"SessionStart":[{"matcher":"[","hooks":[{"type":"command","command":"true"}]}]}}"#,
    );
    repo.commit();
    let malformed = catalog(&repo);
    assert!(has_code(&malformed, "invalid_plugin_hooks"));
    assert!(
        malformed
            .entries()
            .iter()
            .any(|entry| entry.stable_id == "PLUGIN-HOOKS:hooks/hooks.json")
    );

    let repo = TestRepo::new("plugin-default-hook-inactive");
    manifest(&repo);
    repo.write(
        "hooks/hooks.json",
        br#"{"hooks":{"Stop":[{"matcher":"Bash","hooks":[{"type":"prompt","prompt":"fixture"}]}]}}"#,
    );
    repo.commit();
    let inactive = catalog(&repo);
    assert!(has_code(&inactive, "inactive_plugin_hook_configuration"));
    assert!(!has_code(&inactive, "plugin_hook_trust_required"));
    assert!(inactive.entries().iter().any(|entry| {
        entry.stable_id == "PLUGIN-HOOKS:hooks/hooks.json"
            && entry.active_status == ActiveStatus::ContextOnly
    }));
}

#[test]
fn absent_default_hooks_add_no_synthetic_surface() {
    let repo = TestRepo::new("plugin-default-hook-absent");
    manifest(&repo);
    repo.commit();
    let catalog = catalog(&repo);
    assert!(!has_code(&catalog, "invalid_plugin_hooks"));
    assert!(!has_code(&catalog, "plugin_hook_trust_required"));
    assert!(
        !catalog
            .entries()
            .iter()
            .any(|entry| entry.stable_id.starts_with("PLUGIN-HOOKS:"))
    );
}

#[test]
fn explicit_default_path_is_cataloged_exactly_once_without_shadowing_itself() {
    let repo = TestRepo::new("plugin-default-hook-explicit-overlap");
    repo.write(
        ".codex-plugin/plugin.json",
        br#"{"name":"fixture-plugin","version":"1.0.0","description":"fixture","hooks":"./hooks/hooks.json"}"#,
    );
    repo.write(
        "hooks/hooks.json",
        br#"{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"true"}]}]}}"#,
    );
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "unsupported_plugin_manifest_field"));
    assert_eq!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.stable_id == "PLUGIN-HOOKS:hooks/hooks.json")
            .count(),
        1
    );
    assert!(!has_code(&catalog, "plugin_default_hooks_shadowed"));
}

#[cfg(unix)]
#[test]
fn escaping_default_hook_symlinks_are_not_read_or_cataloged() {
    let repo = TestRepo::new("plugin-default-hook-symlink");
    manifest(&repo);
    std::fs::create_dir_all(repo.root.join("hooks")).unwrap();
    std::os::unix::fs::symlink("/dev/null", repo.root.join("hooks/hooks.json")).unwrap();
    repo.commit();
    let catalog = catalog(&repo);
    assert!(has_code(&catalog, "symlink_path_escape"));
    assert!(
        !catalog
            .entries()
            .iter()
            .any(|entry| entry.stable_id == "PLUGIN-HOOKS:hooks/hooks.json")
    );
}
