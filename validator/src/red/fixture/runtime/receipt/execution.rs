use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) fn value(
    root: &Path,
    run_id: &str,
    validator_artifacts: &[Value],
    input_digest_rows: &[Value],
    digest_cache: &mut BTreeMap<String, String>,
) -> Value {
    let now = crate::audit::clock::now_iso();
    json!({
        "producer_actor_id": "package-author",
        "validator_actor_id": "ultragoal-rust-validator",
        "actor_disjoint": true,
        "executable_provenance": {
            "path": "target/debug/ultragoal",
            "digest": crate::red::fixture::runtime::artifact::digest_or_zero_cached(root, "target/debug/ultragoal", digest_cache),
            "source_artifact_set_digest": crate::audit::receipt::source_artifact_set_digest(validator_artifacts),
            "invocation_mode": "direct_executable"
        },
        "command": {
            "id": "ultragoal-audit",
            "command": "target/debug/ultragoal --root . source audit --receipt validation_artifacts/ultragoal-audit/validator-receipt.json --red-report validation_artifacts/ultragoal-audit/red-fixture-report.json",
            "cwd": crate::audit::receipt::root_identity(root),
            "exit": 0,
            "started_at": now,
            "completed_at": now
        },
        "validator_artifacts": validator_artifacts,
        "input_digests": input_digest_rows,
        "stdout": crate::red::fixture::runtime::artifact::ref_value_cached(root, "validation_artifacts/ultragoal-audit/validator-receipt.stdout.txt", digest_cache),
        "stderr": crate::red::fixture::runtime::artifact::ref_value_cached(root, "validation_artifacts/ultragoal-audit/validator-receipt.stderr.txt", digest_cache),
        "started_at": now,
        "completed_at": now,
        "environment": format!("red-fixture-runtime-binding; validator_run_id={run_id}")
    })
}
