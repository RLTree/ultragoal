use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const RAW: &str = "validation_artifacts/ultragoal-audit/active-registry-observation-current.json";

fn setup(label: &str) -> (PathBuf, Value, crate::schema_catalog::SchemaStore) {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(label);
    super::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    let current = crate::package::inventory::package_digest(&root).expect("package digest");
    super::write_json(
        &root.join(RAW),
        &super::fail_closed_raw_observation(&current),
    );
    let digest = crate::digest::file(&root.join(RAW)).expect("raw digest");
    let receipt = super::fail_closed_registry_receipt(&current, &digest);
    let store = crate::schema_catalog::load(
        &crate::self_tests::boundaries::workspace_fixtures::repo_root(),
    );
    (root, receipt, store)
}

fn failures(
    root: &Path,
    receipt: &Value,
    store: &crate::schema_catalog::SchemaStore,
) -> Vec<String> {
    crate::audit::plugin::registry::value_failures(root, store, receipt)
}

#[test]
fn fail_closed_raw_reader_rejects_hardlinks_and_duplicate_json_without_echo() {
    let (root, receipt, store) = setup("plugin-registry-raw-hardlink");
    let raw = root.join(RAW);
    let sibling = root.join("raw-hardlink-source.json");
    fs::rename(&raw, &sibling).expect("move raw");
    fs::hard_link(&sibling, &raw).expect("hardlink raw");
    let errors = failures(&root, &receipt, &store);
    assert!(
        errors
            .iter()
            .any(|error| error == "plugin_self_law_registry_raw_observation_unavailable"),
        "{errors:?}"
    );
    fs::remove_file(&raw).expect("remove hardlink");
    fs::remove_file(&sibling).expect("remove source");

    let canary = "SECRET_CANARY_RAW_READER";
    fs::write(&raw, format!(r#"{{"status":"fail","status":"{canary}"}}"#))
        .expect("write duplicate JSON");
    let errors = failures(&root, &receipt, &store);
    assert!(
        errors
            .iter()
            .any(|error| error == "plugin_self_law_registry_raw_observation_malformed"),
        "{errors:?}"
    );
    assert!(!errors.join("\n").contains(canary), "{errors:?}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn fail_closed_raw_reader_rejects_symlink_ancestor_and_after_read_change() {
    let (root, receipt, store) = setup("plugin-registry-raw-ancestor");
    let audit = root.join("validation_artifacts/ultragoal-audit");
    let moved = root.join("audit-real");
    fs::rename(&audit, &moved).expect("move audit directory");
    std::os::unix::fs::symlink(&moved, &audit).expect("symlink audit directory");
    let errors = failures(&root, &receipt, &store);
    assert!(
        errors
            .iter()
            .any(|error| error == "plugin_self_law_registry_raw_observation_unavailable"),
        "{errors:?}"
    );
    fs::remove_file(&audit).expect("remove symlink");
    fs::rename(&moved, &audit).expect("restore audit directory");

    let changed = root.join(RAW);
    crate::package::inventory::anchored::test_hooks::set_after_read(RAW, move || {
        fs::write(&changed, b"{}").expect("change raw after read");
    });
    let errors = failures(&root, &receipt, &store);
    assert!(
        errors.iter().any(|error| {
            error == "plugin_self_law_registry_raw_observation_unavailable"
                || error == "plugin_self_law_registry_raw_observation_changed_during_read"
        }),
        "{errors:?}"
    );
    fs::remove_dir_all(root).expect("cleanup");
}
