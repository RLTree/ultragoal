#![allow(dead_code, unused_imports)]

#[path = "../src/migration/mod.rs"]
mod migration;

#[path = "migration_product_contract/adversarial.rs"]
mod adversarial;
#[path = "migration_product_contract/positive.rs"]
mod positive;
#[path = "migration_product_contract/recovery.rs"]
mod recovery;
#[path = "migration_product_contract/support.rs"]
mod support;

#[test]
fn corrective_worker_result_uses_the_authoritative_worker_result_v1_shape() {
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
    for artifact in &result.artifacts {
        let path = repository_root.join(&artifact.path);
        let metadata = fs::symlink_metadata(&path).unwrap();
        assert!(metadata.file_type().is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(metadata.nlink(), 1);
        let bytes = fs::read(path).unwrap();
        assert_eq!(bytes.len() as u64, artifact.byte_length);
        let sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
        assert_eq!(sha256, artifact.sha256);
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
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
}
