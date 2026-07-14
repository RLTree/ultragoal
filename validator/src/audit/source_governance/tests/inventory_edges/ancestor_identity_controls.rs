use super::SourceRoot;
use std::path::{Path, PathBuf};

const REGISTRY: &str = "migration/generated-surface-authority.json";
const SHARD: &str = "migration/generated-surface-authority/fixture.json";
const GENERATOR: &str = "scripts/project-generated-authority";
const SOURCE: &str = "agent-standards/policy/fixture.json";
const OUTPUT: &str = "agent-standards/enforcement.json";

#[test]
fn ordinary_directory_ancestor_replacement_with_original_leaves_is_rejected() {
    for (label, ancestor, observed, registry_guard) in [
        ("registry", "migration", REGISTRY, true),
        (
            "shard",
            "migration/generated-surface-authority",
            SHARD,
            false,
        ),
        ("generator", "scripts", GENERATOR, false),
        ("canonical-source", "agent-standards/policy", SOURCE, false),
        ("output", "agent-standards", OUTPUT, false),
    ] {
        let root = SourceRoot::new(&format!("directory-identity-{label}"));
        let before = super::super::fixture::snapshot(root.path());
        let inventory = super::super::super::inventory::capture_once_for_test(root.path())
            .expect("initial governed inventory");
        let mut substitution = None;
        let failures = super::super::super::generated::projection_failures_with_between(
            root.path(),
            &inventory.sources,
            || {
                substitution = Some(DirectorySubstitution::begin(&root.path().join(ancestor)));
            },
        );
        substitution.expect("active substitution").restore();
        assert!(
            failures.contains(&format!(
                "generated_source_projection_revalidation_failed:{observed}:authority_file_ancestor_identity_changed"
            )),
            "{label}: {failures:?}"
        );
        if registry_guard {
            assert!(
                failures.contains(
                    &"generated_source_registry_revalidation_failed:authority_file_ancestor_identity_changed"
                        .to_string()
                ),
                "{label}: {failures:?}"
            );
        }
        assert_eq!(super::super::fixture::snapshot(root.path()), before);
    }
}

#[test]
fn governed_inventory_rejects_ordinary_ancestor_replacement_with_original_leaves() {
    let root = SourceRoot::new("inventory-directory-identity");
    let before = super::super::fixture::snapshot(root.path());
    let mut substitution = None;
    let result = super::super::super::inventory::capture_with_between_for_test(root.path(), || {
        substitution = Some(DirectorySubstitution::begin(
            &root.path().join("validator/src"),
        ));
    });
    substitution.expect("active substitution").restore();
    assert_eq!(
        result.expect_err("ancestor substitution must invalidate the inventory"),
        vec!["governed_source_inventory_changed_during_capture"]
    );
    assert_eq!(super::super::fixture::snapshot(root.path()), before);
}

#[test]
fn ordinary_repository_root_replacement_with_original_tree_is_rejected() {
    let root = SourceRoot::new("repository-root-identity");
    let before = super::super::fixture::snapshot(root.path());
    let inventory = super::super::super::inventory::capture_once_for_test(root.path())
        .expect("initial governed inventory");
    let mut substitution = None;
    let failures = super::super::super::generated::projection_failures_with_between(
        root.path(),
        &inventory.sources,
        || substitution = Some(DirectorySubstitution::begin(root.path())),
    );
    substitution.expect("active substitution").restore();
    assert!(
        failures.contains(&format!(
            "generated_source_projection_revalidation_failed:{REGISTRY}:authority_file_root_identity_changed"
        )),
        "{failures:?}"
    );
    assert!(
        failures.contains(
            &"generated_source_registry_revalidation_failed:authority_file_root_identity_changed"
                .to_string()
        ),
        "{failures:?}"
    );
    assert_eq!(super::super::fixture::snapshot(root.path()), before);
}

struct DirectorySubstitution {
    active: PathBuf,
    original: PathBuf,
}

impl DirectorySubstitution {
    fn begin(active: &Path) -> Self {
        let name = active
            .file_name()
            .and_then(|value| value.to_str())
            .expect("directory name");
        let original = active.with_file_name(format!("{name}-authority-original"));
        let permissions = std::fs::metadata(active)
            .expect("ancestor metadata")
            .permissions();
        std::fs::rename(active, &original).expect("displace original directory");
        std::fs::create_dir(active).expect("create ordinary replacement directory");
        std::fs::set_permissions(active, permissions).expect("match directory mode");
        move_children(&original, active);
        Self {
            active: active.to_path_buf(),
            original,
        }
    }

    fn restore(self) {
        move_children(&self.active, &self.original);
        std::fs::remove_dir(&self.active).expect("remove replacement directory");
        std::fs::rename(&self.original, &self.active).expect("restore original directory");
    }
}

fn move_children(source: &Path, destination: &Path) {
    let mut entries = std::fs::read_dir(source)
        .expect("read directory children")
        .map(|entry| entry.expect("directory child").path())
        .collect::<Vec<_>>();
    entries.sort();
    for entry in entries {
        let name = entry.file_name().expect("child name");
        std::fs::rename(&entry, destination.join(name)).expect("move original child");
    }
}
