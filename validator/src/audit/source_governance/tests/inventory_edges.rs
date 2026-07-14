use super::fixture::{SourceRoot, lines};
use crate::audit::source_governance::scope::SourceClass;
use std::collections::BTreeSet;

mod ancestor_identity_controls;
mod generated_authority;
mod generated_authority_controls;
mod leaf_identity_controls;
mod live_root_documents;

#[test]
fn complete_inventory_covers_each_governed_authored_class() {
    let root = SourceRoot::new("complete");
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let classes = inventory
        .sources
        .iter()
        .map(|source| source.class)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        classes,
        BTreeSet::from([
            SourceClass::ActiveDocumentation,
            SourceClass::AgentStandard,
            SourceClass::BuildConfiguration,
            SourceClass::LiveStandard,
            SourceClass::LiveRootDocument,
            SourceClass::MigrationRegistry,
            SourceClass::PackageTemplate,
            SourceClass::PluginAgent,
            SourceClass::PluginConfiguration,
            SourceClass::PluginSkill,
            SourceClass::RepoCheck,
            SourceClass::RustBuild,
            SourceClass::RustBuildSupport,
            SourceClass::RustExample,
            SourceClass::RustProduction,
            SourceClass::RustTest,
            SourceClass::Schema,
        ])
    );
}

#[test]
fn removing_a_required_governed_class_fails_inventory() {
    let root = SourceRoot::new("missing-class");
    std::fs::remove_dir_all(root.path().join("validator/examples")).expect("remove examples");
    let failures =
        crate::audit::source_governance::capture(root.path()).expect_err("missing class");
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("validator/examples")),
        "{failures:?}"
    );
}

#[test]
fn untracked_over_cap_sources_fail_in_every_rust_scope_and_new_script_names_are_governed() {
    let root = SourceRoot::new("caps");
    for relative in [
        "validator/src/oversize.rs",
        "validator/tests/oversize.rs",
        "validator/build_support/oversize.rs",
        "validator/examples/oversize.rs",
    ] {
        root.write(relative, &lines(251));
    }
    root.executable("scripts/run-product-audit", &lines(251));
    root.write("skills/oversize/SKILL.md", &lines(251));
    root.write("agents/oversize-agent.md", &lines(251));
    root.write("agent-standards/policy/oversize.json", &lines(251));
    root.write("schemas/machine-catalog.json", &lines(251));
    root.write("templates/agent-standards/oversize-policy.md", &lines(251));
    let inventory = super::super::inventory::capture_once_for_test(root.path())
        .expect("raw governed inventory");
    let failures = crate::audit::source_governance::line_cap_failures(&inventory);
    for relative in [
        "validator/src/oversize.rs",
        "validator/tests/oversize.rs",
        "validator/build_support/oversize.rs",
        "validator/examples/oversize.rs",
        "scripts/run-product-audit",
        "skills/oversize/SKILL.md",
        "agents/oversize-agent.md",
        "agent-standards/policy/oversize.json",
        "schemas/machine-catalog.json",
        "templates/agent-standards/oversize-policy.md",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(relative)),
            "{relative}: {failures:?}"
        );
    }
}

#[cfg(unix)]
#[test]
fn symlinks_and_special_files_fail_closed() {
    use std::os::unix::fs::symlink;
    use std::process::Command;
    let root = SourceRoot::new("special");
    symlink(
        root.path().join("validator/src/lib.rs"),
        root.path().join("validator/src/alias.rs"),
    )
    .expect("symlink");
    let fifo = root.path().join("validator/tests/event.rs");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo")
            .success()
    );
    let failures = crate::audit::source_governance::capture(root.path()).expect_err("rejected");
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("symlink_rejected")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("special_file_rejected")),
        "{failures:?}"
    );
}

#[cfg(unix)]
#[test]
fn unreadable_governed_entry_is_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let root = SourceRoot::new("unreadable");
    let path = root.path().join("validator/src/lib.rs");
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&path, permissions).expect("remove read permission");
    let result = crate::audit::source_governance::capture(root.path());
    let mut restore = std::fs::metadata(&path).expect("metadata").permissions();
    restore.set_mode(0o600);
    std::fs::set_permissions(path, restore).expect("restore permission");
    assert!(
        result
            .expect_err("unreadable rejected")
            .iter()
            .any(|failure| failure.contains("unreadable"))
    );
}

#[test]
fn inventory_replacement_between_snapshots_is_rejected() {
    let root = SourceRoot::new("race");
    let source = root.path().join("validator/src/lib.rs");
    let result = super::super::inventory::capture_with_between_for_test(root.path(), || {
        std::fs::write(source, "pub fn replaced() {}\n").expect("replace source");
    });
    assert_eq!(
        result.expect_err("changed"),
        vec!["governed_source_inventory_changed_during_capture"]
    );
}

#[test]
fn thin_cargo_test_route_is_not_partial_factoring_but_junk_support_path_is_rejected() {
    let paths = vec![
        "validator/tests/domain_contract.rs".to_string(),
        "validator/tests/domain_contract/behavior.rs".to_string(),
    ];
    let topology =
        crate::audit::namespace::source::topology::failures_with_repo_paths(&paths, &paths);
    assert!(
        !topology
            .iter()
            .any(|failure| failure.contains("partial_module_factoring")),
        "{topology:?}"
    );
    let junk = crate::audit::namespace::law::path_name_failures_for_test(&[
        "validator/tests/domain_contract/helpers/behavior.rs".to_string(),
    ]);
    assert!(
        junk.iter()
            .any(|failure| failure.contains("namespace_junk_drawer_path")),
        "{junk:?}"
    );
}

#[test]
fn scope_exclusions_are_exact_typed_and_explained() {
    use crate::audit::source_governance::scope::EXCLUSIONS;
    let mut paths = BTreeSet::new();
    for exclusion in EXCLUSIONS {
        assert!(!exclusion.relative.contains('*'));
        assert!(exclusion.relative.ends_with('/'));
        assert!(exclusion.rationale.len() >= 24);
        assert!(paths.insert(exclusion.relative));
        let _typed_kind = exclusion.kind;
    }
}
