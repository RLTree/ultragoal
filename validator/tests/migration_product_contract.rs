#[path = "../src/migration/mod.rs"]
mod migration;

#[path = "migration_product_contract/adversarial/mod.rs"]
mod adversarial;
#[path = "migration_product_contract/positive/mod.rs"]
mod positive;
#[path = "migration_product_contract/recovery/mod.rs"]
mod recovery;
#[path = "migration_product_contract/runtime_fixtures/mod.rs"]
mod runtime_fixtures;

#[test]
fn corrective_worker_result_remains_structural_context_after_source_topology_changes() {
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::os::unix::fs::MetadataExt;
    use std::path::Path;

    let bytes = include_bytes!(
        "../../docs/ultragoal-successor-live/worker-results/MIGRATION-RETIREMENT-PRODUCTION-090.json"
    );
    let result = ultragoal::orchestration::WorkerResultV1::parse_json(bytes)
        .expect("authoritative WorkerResultV1 parse");
    assert_eq!(result.worker, "/root/routine_execution_authority_reviewer");
    assert_eq!(result.lease_id, "MIGRATION-COMPATIBILITY-BOUNDARY-100");
    assert_eq!(result.artifacts.len(), 9);
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut aggregate_rows = Vec::new();
    let mut stale_artifact = false;
    for artifact in &result.artifacts {
        let path = repository_root.join(&artifact.path);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            stale_artifact = true;
            aggregate_rows.push(format!(
                "{}\t{}\n",
                artifact.path,
                artifact.sha256.strip_prefix("sha256:").unwrap()
            ));
            continue;
        };
        assert!(metadata.file_type().is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(metadata.nlink(), 1);
        let bytes = fs::read(path).unwrap();
        stale_artifact |= bytes.len() as u64 != artifact.byte_length;
        let sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
        stale_artifact |= sha256 != artifact.sha256;
        aggregate_rows.push(format!(
            "{}\t{}\n",
            artifact.path,
            artifact.sha256.strip_prefix("sha256:").unwrap()
        ));
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
    assert!(stale_artifact);
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
}
