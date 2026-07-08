use super::fixture_workspace::{
    assert_row, cleanup, temp_root, write_canonical_bin, write_file, write_manifest,
};

#[test]
fn inline_cfg_test_modules_do_not_leak_validation_state() {
    let root = temp_root("inline-cfg-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
#[cfg(test)]
mod validation_checks {
    pub fn inside_validation_module() {}
}

pub fn production_after_validation_module() {}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/scheduler/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::validation_checks::inside_validation_module",
        "function",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::production_after_validation_module",
        "function",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn one_line_cfg_test_modules_do_not_hide_following_product_surfaces() {
    let root = temp_root("one-line-cfg-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
#[cfg(test)]
mod validation_checks {}

pub fn production_after_empty_validation_module() {}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/scheduler/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::production_after_empty_validation_module",
        "function",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn cfg_test_module_closed_on_function_line_does_not_hide_following_product_surfaces() {
    let root = temp_root("function-line-closes-cfg-test-module");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
#[cfg(test)]
mod validation_checks {
    pub fn inside_validation_module() {} }

pub fn production_after_compact_validation_module() {}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/scheduler/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::validation_checks::inside_validation_module",
        "function",
        "test_only_validation_surface",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::production_after_compact_validation_module",
        "function",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn malformed_dirty_function_declaration_does_not_stop_symbol_scan() {
    let root = temp_root("dirty-malformed-function-symbol");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
fn <dirty_edit_in_progress>

pub fn production_after_dirty_line() {}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/scheduler/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::production_after_dirty_line",
        "function",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn inline_product_modules_preserve_symbol_ownership() {
    let root = temp_root("inline-product-module-symbols");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/cli/standards/mod.rs",
        r#"
mod claim_projection {
    pub fn text() {}
    pub struct StdoutContract;
    impl StdoutContract {
        pub fn render() {}
    }
}

mod command_stdout {
    pub fn text() {}
    pub enum Format {
        Plain,
        Json,
    }
}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/cli/standards/mod.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/cli/standards/mod.rs::claim_projection",
        "inline_module",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/cli/standards/mod.rs::claim_projection::text",
        "function",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/cli/standards/mod.rs::command_stdout::text",
        "function",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/cli/standards/mod.rs::claim_projection::StdoutContract::render",
        "function",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/cli/standards/mod.rs::command_stdout::Format::Plain",
        "enum_variant",
        "canonical",
    );
    cleanup(root);
}
