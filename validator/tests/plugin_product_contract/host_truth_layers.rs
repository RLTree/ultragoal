use super::plugin_product::product_fitness::{
    ClaimCeiling, TRUTH_LAYERS, TruthLayer, source_candidate_ceilings,
};

#[test]
fn source_package_and_all_host_truth_layers_remain_independent() {
    assert_eq!(
        TRUTH_LAYERS,
        [
            TruthLayer::Source,
            TruthLayer::Package,
            TruthLayer::Marketplace,
            TruthLayer::Install,
            TruthLayer::Cache,
            TruthLayer::AppRegistry,
            TruthLayer::PluginsUi,
            TruthLayer::Discovery,
            TruthLayer::Runtime,
            TruthLayer::Journey,
        ]
    );
    let ceilings = source_candidate_ceilings();
    assert!(
        TRUTH_LAYERS
            .iter()
            .all(|layer| ceilings[layer] == ClaimCeiling::Withheld)
    );
}

#[test]
fn source_docs_do_not_contain_claim_collapsing_language() {
    let source = format!(
        "{}\n{}",
        super::read("docs/install-and-visibility.md"),
        super::read("docs/plugin-resource-map.md")
    );
    let source = source.split_whitespace().collect::<Vec<_>>().join(" ");
    for marker in [
        "Package, marketplace, install, cache, app registry, Plugins UI, discovery, and runtime are not synonyms.",
        "Product Fitness is independently withheld",
        "authorized rollback",
        "stale-cache recovery",
        "uninstall and teardown",
    ] {
        assert!(source.contains(marker), "missing marker: {marker}");
    }
}
