use super::fixture_workspace::{
    assert_contains, cleanup, failures, inventory, temp_root, write_canonical_bin, write_file,
};

#[cfg(unix)]
#[test]
fn package_tree_scan_error_fails_closed() {
    let root = temp_root("package-scan-error");
    write_canonical_bin(&root);
    let private = root.join("validator/src/private");
    std::fs::create_dir_all(&private).expect("private dir");
    make_unreadable(&private);

    let failures = failures(&root, inventory(&["validator/src/bin/ultragoal.rs"]));
    make_readable(&private);
    assert_contains(&failures, "surface=package-tree-scan");
    assert_contains(&failures, "surface=package-resource-scan");
    cleanup(root);
}

#[cfg(unix)]
#[test]
fn unreadable_source_symbols_fail_closed() {
    let root = temp_root("unreadable-source-symbols");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/source_topology.rs",
        "pub fn route() {}\n",
    );
    make_unreadable(&root.join("validator/src/cli/source_topology.rs"));

    let failures = failures(
        &root,
        inventory(&[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/source_topology.rs",
        ]),
    );
    make_readable(&root.join("validator/src/cli/source_topology.rs"));
    assert_contains(
        &failures,
        "surface=source-symbol-read:validator/src/cli/source_topology.rs",
    );
    cleanup(root);
}

#[cfg(unix)]
fn make_unreadable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000))
        .expect("make unreadable");
}

#[cfg(unix)]
fn make_readable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}
