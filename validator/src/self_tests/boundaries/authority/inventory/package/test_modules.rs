use super::fixture_workspace::{
    assert_row, cleanup, temp_root, write_canonical_bin, write_file, write_manifest,
};

#[test]
fn out_of_line_test_modules_are_validation_surfaces() {
    let root = temp_root("out-of-line-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/observe/command_roundtrip/tests/mod.rs",
        "pub(crate) fn roundtrip_root() {}\n",
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/observe/command_roundtrip/tests/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/cli/observe/command_roundtrip/tests/mod.rs",
        "rust_module",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/cli/observe/command_roundtrip/tests/mod.rs::roundtrip_root",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}

#[test]
fn cfg_test_sibling_modules_are_validation_surfaces() {
    let root = temp_root("cfg-test-sibling-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/typed_boundaries/mod.rs",
        "#[cfg(test)]\nmod receipt_checks;\n",
    );
    write_file(
        &root,
        "validator/src/cli/typed_boundaries/receipt_checks.rs",
        "pub(crate) fn receipt_summary_contract() {}\n",
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/typed_boundaries/mod.rs",
            "validator/src/cli/typed_boundaries/receipt_checks.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/cli/typed_boundaries/receipt_checks.rs",
        "rust_module",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/cli/typed_boundaries/receipt_checks.rs::receipt_summary_contract",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}

#[test]
fn crate_root_cfg_test_sibling_modules_are_validation_surfaces() {
    let root = temp_root("crate-root-cfg-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/lib.rs",
        "#[cfg(test)]\nmod root_checks;\n",
    );
    write_file(
        &root,
        "validator/src/root_checks.rs",
        "pub(crate) fn package_surface_contract() {}\n",
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/lib.rs",
            "validator/src/root_checks.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/root_checks.rs",
        "rust_module",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/root_checks.rs::package_surface_contract",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}

#[test]
fn cfg_test_directory_modules_are_validation_surfaces() {
    let root = temp_root("cfg-test-directory-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/live_loop/nodes/measurement/mod.rs",
        "#[cfg(test)]\nmod command;\n",
    );
    write_file(
        &root,
        "validator/src/cli/live_loop/nodes/measurement/command/mod.rs",
        "pub(crate) fn command_receipt_contract() {}\n",
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/live_loop/nodes/measurement/mod.rs",
            "validator/src/cli/live_loop/nodes/measurement/command/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/cli/live_loop/nodes/measurement/command/mod.rs",
        "rust_module",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/cli/live_loop/nodes/measurement/command/mod.rs::command_receipt_contract",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}

#[test]
fn nested_modules_under_cfg_test_roots_remain_validation_surfaces() {
    let root = temp_root("nested-cfg-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/lib.rs",
        "#[cfg(test)]\nmod validation;\n",
    );
    write_file(
        &root,
        "validator/src/validation/mod.rs",
        "mod command_receipts;\n",
    );
    write_file(
        &root,
        "validator/src/validation/command_receipts.rs",
        "pub(crate) fn receipt_contract() {}\n",
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/lib.rs",
            "validator/src/validation/mod.rs",
            "validator/src/validation/command_receipts.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/validation/command_receipts.rs",
        "rust_module",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/validation/command_receipts.rs::receipt_contract",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}

#[test]
fn path_attributed_modules_under_cfg_test_owners_are_validation_surfaces() {
    let root = temp_root("path-attributed-cfg-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/live_loop/nodes/measurement/observation/mod.rs",
        "#[cfg(test)]\nmod status;\n",
    );
    write_file(
        &root,
        "validator/src/cli/live_loop/nodes/measurement/observation/status.rs",
        "#[path = \"roundtrip_failure_tests.rs\"]\nmod roundtrip_failure;\n",
    );
    write_file(
        &root,
        "validator/src/cli/live_loop/nodes/measurement/observation/roundtrip_failure_tests.rs",
        "fn all_roundtrips() {}\n",
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/live_loop/nodes/measurement/observation/mod.rs",
            "validator/src/cli/live_loop/nodes/measurement/observation/status.rs",
            "validator/src/cli/live_loop/nodes/measurement/observation/roundtrip_failure_tests.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/cli/live_loop/nodes/measurement/observation/roundtrip_failure_tests.rs",
        "rust_module",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/cli/live_loop/nodes/measurement/observation/roundtrip_failure_tests.rs::all_roundtrips",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}
