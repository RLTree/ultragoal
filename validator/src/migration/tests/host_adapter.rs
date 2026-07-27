#[path = "../../../tests/migration_host_adapter_contract/adversarial/mod.rs"]
mod adversarial;
#[path = "../../../tests/migration_host_adapter_contract/host_fixture/mod.rs"]
mod host_fixture;
#[path = "../../../tests/migration_host_adapter_contract/positive.rs"]
mod positive;
#[path = "../../../tests/migration_host_adapter_contract/recovery.rs"]
mod recovery;

#[test]
fn darwin_migration_host_adapter_is_wired_to_the_accepted_product_boundary() {
    use crate::migration::product::DarwinMigrationHost;
    let _ = std::mem::size_of::<DarwinMigrationHost>();
}

#[test]
fn migration_host_worker_result_uses_the_authoritative_shape() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/ultragoal-successor-live/worker-results/MIGRATION-HOST-PRODUCTION-101.json");
    let bytes = std::fs::read(path).unwrap();
    let result = crate::orchestration::WorkerResultV1::parse_json(&bytes).unwrap();
    assert_eq!(result.lease_id, "MIGRATION-HOST-PRODUCTION-101");
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
}
