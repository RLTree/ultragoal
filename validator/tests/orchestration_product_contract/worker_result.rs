use serde_json::Value;
use std::collections::BTreeSet;
use ultragoal::orchestration::{LeaseSpec, ScopePolicy, WorkPackage, WorkerResultV1};

const ENVELOPE: &str = include_str!(
    "../../../docs/ultragoal-successor-live/work-packages/ORCHESTRATION-RECOVERY-PRODUCT-CORRECTION-029.json"
);
const RESULT: &str = include_str!(
    "../../../docs/ultragoal-successor-live/worker-results/ORCHESTRATION-RECOVERY-PRODUCT-CORRECTION-029.json"
);

#[test]
fn historical_worker_result_remains_structurally_lease_bound() {
    let envelope: Value = serde_json::from_str(ENVELOPE).unwrap();
    assert_eq!(envelope["schema_version"], "RootIssuedWorkEnvelope-v1");
    let package: WorkPackage = serde_json::from_value(envelope["work_package"].clone()).unwrap();
    let lease: LeaseSpec = serde_json::from_value(envelope["lease"].clone()).unwrap();
    package.validate().unwrap();
    let policy = ScopePolicy {
        allowed_read_paths: lease.read_paths.clone(),
        allowed_paths: lease.owned_scope.paths.clone(),
        allowed_semantic_prefixes: lease.owned_scope.semantic_symbols.clone(),
        allowed_generated_outputs: lease.owned_scope.generated_outputs.clone(),
        allowed_fixtures: lease.owned_scope.fixtures.clone(),
        allowed_effects: lease.owned_scope.effects.clone(),
        ..ScopePolicy::default()
    };
    lease.validate(&policy).unwrap();
    let result = WorkerResultV1::parse_json(RESULT.as_bytes()).unwrap();
    result.validate_for(&lease, &package).unwrap();

    let result_path = "docs/ultragoal-successor-live/worker-results/ORCHESTRATION-RECOVERY-PRODUCT-CORRECTION-029.json";
    let touched = result
        .touched_paths
        .iter()
        .filter(|path| path.as_str() != result_path)
        .cloned()
        .collect::<BTreeSet<_>>();
    let artifacts = result
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(artifacts, touched);
    assert_eq!(
        result.worker.as_str(),
        "/root/orchestration_recovery_product_engineer"
    );
    assert_eq!(
        result.lease_id.as_str(),
        "ORCHESTRATION-RECOVERY-PRODUCT-CORRECTION-029"
    );
    assert_eq!(
        result.no_claim_statement,
        "This worker does not claim readiness, release, or completion."
    );
}
