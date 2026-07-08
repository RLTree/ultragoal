use super::fixture_workspace::{
    assert_row, assert_surface_id, cleanup, temp_root, write_canonical_bin, write_file,
    write_manifest,
};

#[test]
fn same_named_methods_keep_impl_context_in_surface_ids() {
    let root = temp_root("same-named-method-surfaces");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/product/manifest.rs",
        r#"
pub(crate) struct PluginFlowEdge;
pub(crate) struct PluginFlowStage;

impl PluginFlowEdge {
    pub(crate) fn from_value() -> Self {
        Self
    }
}

impl PluginFlowStage {
    pub(crate) fn from_value() -> Self {
        Self
    }
}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/product/manifest.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowEdge::from_value",
        "function",
        "canonical",
    );
    assert_surface_id(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowEdge::from_value",
        "function:validator-src-product-manifest-rs--pluginflowedge--from-value",
    );
    assert_row(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowStage::from_value",
        "function",
        "canonical",
    );
    assert_surface_id(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowStage::from_value",
        "function:validator-src-product-manifest-rs--pluginflowstage--from-value",
    );
    cleanup(root);
}

#[test]
fn generic_impl_methods_keep_product_type_context() {
    let root = temp_root("generic-impl-method-surface");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/product/manifest.rs",
        r#"
pub(crate) struct PluginFlowStage<T>(T);

impl<T> PluginFlowStage<T> {
    pub(crate) fn from_value(value: T) -> Self {
        Self(value)
    }
}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/product/manifest.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowStage::from_value",
        "function",
        "canonical",
    );
    assert_surface_id(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowStage::from_value",
        "function:validator-src-product-manifest-rs--pluginflowstage--from-value",
    );
    cleanup(root);
}

#[test]
fn unclosed_generic_impl_header_stays_readable_during_dirty_edits() {
    let root = temp_root("dirty-generic-impl-method-surface");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/product/manifest.rs",
        r#"
pub(crate) struct PluginFlowStage<T>(T);

impl<T PluginFlowStage<T> {
    pub(crate) fn from_value(value: T) -> Self {
        Self(value)
    }
}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/product/manifest.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/product/manifest.rs::from_value",
        "function",
        "canonical",
    );
    cleanup(root);
}

#[test]
fn cfg_test_impl_methods_remain_validation_surfaces() {
    let root = temp_root("cfg-test-impl-method-surface");
    write_canonical_bin(&root);
    write_file(
        &root,
        "validator/src/product/manifest.rs",
        r#"
pub(crate) struct PluginFlowStage;

#[cfg(test)]
impl PluginFlowStage {
    pub(crate) fn fixture_only_constructor() -> Self {
        Self
    }
}
"#,
    );
    write_manifest(
        &root,
        &[
            "validator/src/bin/ultragoal.rs",
            "validator/src/product/manifest.rs",
        ],
    );

    let inventory =
        crate::audit::law::authority_surfaces::package_surface_inventory_artifact(&root);
    assert_row(
        &inventory,
        "validator/src/product/manifest.rs::PluginFlowStage::fixture_only_constructor",
        "function",
        "test_only_validation_surface",
    );
    cleanup(root);
}
