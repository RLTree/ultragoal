#[path = "../src/migration/mod.rs"]
mod migration;

#[path = "migration_host_adapter_contract/adversarial.rs"]
mod adversarial;
#[path = "migration_host_adapter_contract/host_fixture.rs"]
mod host_fixture;
#[path = "migration_host_adapter_contract/positive.rs"]
mod positive;
#[path = "migration_host_adapter_contract/recovery.rs"]
mod recovery;

#[test]
fn darwin_migration_host_adapter_is_wired_to_the_accepted_product_boundary() {
    use migration::product::DarwinMigrationHost;
    let _ = std::mem::size_of::<DarwinMigrationHost>();
}

#[test]
fn migration_host_worker_result_uses_the_authoritative_shape() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/ultragoal-successor-live/worker-results/MIGRATION-HOST-PRODUCTION-101.json");
    let bytes = std::fs::read(path).unwrap();
    let result = ultragoal::orchestration::WorkerResultV1::parse_json(&bytes).unwrap();
    assert_eq!(result.lease_id, "MIGRATION-HOST-PRODUCTION-101");
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
}
