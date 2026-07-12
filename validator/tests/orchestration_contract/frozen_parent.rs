use crate::orchestration::*;
use crate::support::*;
use std::collections::BTreeSet;

fn node006_subject() -> (WorkerResultV1, LeaseSpec, WorkPackage, ScopePolicy) {
    let result = WorkerResultV1::parse_json(include_bytes!(
        "../../../docs/ultragoal-successor-live/worker-results/LEASE-N10-ORCHESTRATION-NODE-006.json"
    ))
    .unwrap();
    let binding = Binding::new(
        &result.context_id,
        result.candidate_identity["candidate_id"].as_str().unwrap(),
    )
    .unwrap();
    let generated_outputs: BTreeSet<_> = result
        .generated_outputs
        .iter()
        .map(|value| path(value))
        .collect();
    let fixtures: BTreeSet<_> = result.fixtures.iter().map(|value| path(value)).collect();
    let touched: BTreeSet<_> = result
        .touched_paths
        .iter()
        .map(|value| path(value))
        .collect();
    let read_paths: BTreeSet<CanonicalPath> = result
        .artifacts
        .iter()
        .filter(|artifact| !touched.contains(&path(&artifact.path)))
        .map(|artifact| path(&artifact.path))
        .collect();
    let owned_scope = OwnedScope {
        paths: touched
            .iter()
            .filter(|value| !generated_outputs.contains(*value) && !fixtures.contains(*value))
            .cloned()
            .collect(),
        semantic_symbols: result.touched_semantics.iter().cloned().collect(),
        generated_outputs,
        fixtures,
        effects: result
            .effects
            .iter()
            .filter(|usage| usage.performed)
            .map(|usage| EffectGrant::new(usage.class, &usage.target).unwrap())
            .collect(),
    };
    let package = WorkPackage {
        node_id: "N10-ORCHESTRATION".to_owned(),
        dependencies: result.dependency_nodes.iter().cloned().collect(),
        required_tools: BTreeSet::new(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: read_paths.clone(),
        owned_scope: owned_scope.clone(),
        prerequisites: BTreeSet::new(),
        outputs: BTreeSet::from(["n10-node006-fixture".to_owned()]),
        acceptance: BTreeSet::from(["independent-review".to_owned()]),
        claim_effect: "observation-only".to_owned(),
    };
    let lease = LeaseSpec {
        lease_id: result.lease_id.clone(),
        run_id: "run-n10-node006-fixture".to_owned(),
        node_id: package.node_id.clone(),
        principal: Principal::Worker,
        owner: Actor::parse(&result.worker).unwrap(),
        binding,
        safety_class: package.safety_class,
        read_paths: read_paths.clone(),
        owned_scope: owned_scope.clone(),
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 20,
        max_retries: 2,
    };
    let policy = ScopePolicy {
        allowed_read_paths: read_paths,
        allowed_paths: owned_scope.paths.clone(),
        allowed_semantic_prefixes: owned_scope.semantic_symbols.clone(),
        allowed_generated_outputs: owned_scope.generated_outputs.clone(),
        allowed_fixtures: owned_scope.fixtures.clone(),
        allowed_effects: owned_scope.effects.clone(),
        ..ScopePolicy::default()
    };
    (result, lease, package, policy)
}

#[test]
fn exact_node006_parent_parses_29_artifacts_and_reaches_typed_commitment() {
    let (result, lease, package, policy) = node006_subject();
    let touched: BTreeSet<_> = result.touched_paths.iter().collect();
    assert_eq!(result.artifacts.len(), 29);
    assert_eq!(
        result
            .artifacts
            .iter()
            .filter(|artifact| !touched.contains(&artifact.path))
            .count(),
        26
    );
    lease.validate(&policy).unwrap();
    result.validate_for(&lease, &package).unwrap();
    AcceptanceProposal::result_commitment_id_for(&lease.binding, &package, &lease, &result)
        .unwrap();
}

#[test]
fn node006_parent_rejects_missing_read_authority_and_touch_relabeling() {
    let (result, lease, package, policy) = node006_subject();
    for required_read in [
        "validator/src/orchestration/effect.rs",
        "validator/tests/orchestration_contract/blockers.rs",
    ] {
        let mut incomplete = lease.clone();
        incomplete.read_paths.remove(&path(required_read));
        incomplete.validate(&policy).unwrap();
        assert!(
            result.validate_for(&incomplete, &package).is_err(),
            "missing read root {required_read} must reject read-only evidence"
        );
    }

    let mut relabeled = result.clone();
    relabeled
        .touched_paths
        .retain(|value| value != "validator/src/orchestration/worker.rs");
    assert_eq!(
        relabeled.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}

#[test]
fn node006_parent_mutation_changes_identity_and_stale_fixture_fails() {
    let (mut result, lease, package, _) = node006_subject();
    let original = result.result_id().unwrap();
    result.artifacts[0].byte_length += 1;
    assert_ne!(result.result_id().unwrap(), original);

    result.artifacts[0].path = "validator/src/orchestration/WORKER.rs".to_owned();
    assert_eq!(
        result.validate_for(&lease, &package).unwrap_err(),
        OrchestrationError::InvalidWorkerResult
    );
}
