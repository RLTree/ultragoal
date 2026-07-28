#[test]
fn compile_compatibility_does_not_activate_unsupported_product_apis() {
    let repo = source_repo("activation-unsupported-api");
    let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    for api in [
        "PackageSnapshot",
        "InstallSnapshot",
        "MarketplaceSnapshot",
        "DiscoveryObservation",
        "RuntimeObservation",
        "FixtureScheduler",
        "ExportAdapter",
    ] {
        let stable_id = format!("API:{api}");
        assert!(catalog.entries().iter().any(|entry| {
            entry.stable_id == stable_id
                && entry.generator.as_deref() == Some("HCT-INVENTORY:compiled-api-witness")
                && entry.active_status == ActiveStatus::Definition
        }));
    }
}
