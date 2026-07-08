use super::fixture_workspace::{
    assert_row, assert_surface_id, cleanup, temp_root, write_canonical_bin, write_file,
    write_manifest,
};

#[test]
fn cfg_test_semicolon_module_does_not_hide_product_constants_or_types() {
    let root = temp_root("cfg-test-module-symbols");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
#[cfg(test)]
mod tests;

const MAX_EXPLICIT_JOBS: usize = 256;

pub(crate) enum TaskClass {
    PureReadParallel,
}
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
        "validator/src/scheduler/mod.rs::MAX_EXPLICIT_JOBS",
        "constant",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::TaskClass",
        "type",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn enum_variants_are_product_surfaces() {
    let root = temp_root("enum-variant-symbols");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
pub(crate) enum TaskClass {
    PureReadParallel,
    SharedAuthorityWriteSerial,
}
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
        "validator/src/scheduler/mod.rs::TaskClass::PureReadParallel",
        "enum_variant",
        "canonical",
    );
    assert_surface_id(
        &inventory,
        "validator/src/scheduler/mod.rs::TaskClass::PureReadParallel",
        "enum-variant:validator-src-scheduler-mod-rs--taskclass--purereadparallel",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::TaskClass::SharedAuthorityWriteSerial",
        "enum_variant",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn positive_test_cfg_is_validation_and_not_test_cfg_is_canonical() {
    let root = temp_root("positive-and-negative-test-cfg");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
#[cfg(not(test))]
pub fn production_only() {}

#[cfg(test)]
pub fn validation_only() {}
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
        "validator/src/scheduler/mod.rs::production_only",
        "function",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::validation_only",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}

#[test]
fn cfg_feature_names_do_not_hide_product_surfaces() {
    let root = temp_root("cfg-feature-contest-is-product");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/scheduler/mod.rs",
        r#"
#[cfg(feature = "contest")]
pub fn contest_feature() {}

#[cfg(any(test, feature = "debug-ui"))]
pub fn debug_or_test_feature() {}

#[cfg(all(test, feature = "fixture"))]
pub fn test_fixture_feature() {}
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
        "validator/src/scheduler/mod.rs::contest_feature",
        "function",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::debug_or_test_feature",
        "function",
        "canonical",
    );
    assert_row(
        &inventory,
        "validator/src/scheduler/mod.rs::test_fixture_feature",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}
