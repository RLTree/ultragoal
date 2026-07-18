pub(crate) const PLUGIN_PRODUCT_REVIEW_READERS: &[ReaderSpec] = &[
    // These three production modules jointly read, decode, and validate the
    // current source and host agent-authority catalogs. They are positive
    // readers, so their exact bytes are bound without a negative-only escape.
    reader!(
        "validator/src/plugin_product/agent_discovery/host/mod.rs",
        "../plugin_product/agent_discovery/host/mod.rs",
        false
    ),
    reader!(
        "validator/src/plugin_product/agent_discovery/model.rs",
        "../plugin_product/agent_discovery/model.rs",
        false
    ),
    reader!(
        "validator/src/plugin_product/agent_discovery/source/mod.rs",
        "../plugin_product/agent_discovery/source/mod.rs",
        false
    ),
];
