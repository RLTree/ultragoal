use super::SourceRoot;

const OUTPUT: &str = "agent-standards/enforcement.json";

#[cfg(unix)]
#[test]
fn hard_link_introduced_after_capture_is_rejected_and_removed_cleanly() {
    let root = SourceRoot::new("generated-output-hard-link-race");
    let before = super::super::fixture::snapshot(root.path());
    let inventory = super::super::super::inventory::capture_once_for_test(root.path())
        .expect("initial governed inventory");
    let output = root.path().join(OUTPUT);
    let link = root.path().join("output-authority-hard-link");
    let failures = super::super::super::generated::projection_failures_with_between(
        root.path(),
        &inventory.sources,
        || std::fs::hard_link(&output, &link).expect("create output hard link"),
    );
    std::fs::remove_file(link).expect("remove output hard link");
    assert!(
        failures.contains(&format!(
            "generated_source_projection_revalidation_failed:{OUTPUT}:authority_file_linked_leaf_rejected"
        )),
        "{failures:?}"
    );
    assert_eq!(super::super::fixture::snapshot(root.path()), before);
}

#[cfg(unix)]
#[test]
fn mode_change_after_capture_is_rejected_and_restored_cleanly() {
    use std::os::unix::fs::PermissionsExt;

    let root = SourceRoot::new("generated-output-mode-race");
    let before = super::super::fixture::snapshot(root.path());
    let inventory = super::super::super::inventory::capture_once_for_test(root.path())
        .expect("initial governed inventory");
    let output = root.path().join(OUTPUT);
    let original = std::fs::metadata(&output)
        .expect("output metadata")
        .permissions();
    let original_mode = original.mode();
    let failures = super::super::super::generated::projection_failures_with_between(
        root.path(),
        &inventory.sources,
        || {
            let mut changed = original.clone();
            changed.set_mode(original_mode ^ 0o100);
            std::fs::set_permissions(&output, changed).expect("change output mode");
        },
    );
    std::fs::set_permissions(&output, original).expect("restore output mode");
    assert!(
        failures.contains(&format!(
            "generated_source_projection_revalidation_failed:{OUTPUT}:authority_file_leaf_identity_changed"
        )),
        "{failures:?}"
    );
    assert_eq!(super::super::fixture::snapshot(root.path()), before);
}
