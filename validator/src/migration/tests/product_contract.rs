#[path = "../../../tests/migration_product_contract/adversarial/mod.rs"]
mod adversarial;
#[path = "../../../tests/migration_product_contract/positive/mod.rs"]
mod positive;
#[path = "../../../tests/migration_product_contract/recovery/mod.rs"]
mod recovery;
#[path = "../../../tests/migration_product_contract/runtime_fixtures/mod.rs"]
mod runtime_fixtures;

#[test]
fn corrective_worker_result_remains_structural_context_after_source_topology_changes() {
    use sha2::{Digest, Sha256};

    let bytes = include_bytes!(
        "../../../../docs/ultragoal-successor-live/worker-results/MIGRATION-RETIREMENT-PRODUCTION-090.json"
    );
    let result = crate::orchestration::WorkerResultV1::parse_json(bytes)
        .expect("authoritative WorkerResultV1 parse");
    assert_eq!(result.worker, "/root/routine_execution_authority_reviewer");
    assert_eq!(result.lease_id, "MIGRATION-COMPATIBILITY-BOUNDARY-100");
    assert_eq!(result.artifacts.len(), 9);
    let mut aggregate_rows = Vec::new();
    for artifact in &result.artifacts {
        let digest = artifact.sha256.strip_prefix("sha256:").unwrap();
        assert_eq!(digest.len(), 64);
        assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
        aggregate_rows.push(format!("{}\t{}\n", artifact.path, digest));
    }
    aggregate_rows.sort();
    let aggregate = format!(
        "sha256:{:x}",
        Sha256::digest(aggregate_rows.concat().as_bytes())
    );
    assert_eq!(
        result.candidate_identity["correction_artifact_set_sha256"],
        aggregate
    );
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
}
