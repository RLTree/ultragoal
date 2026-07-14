use super::*;

pub(crate) fn r3_artifact_set_is_exact(result: &WorkerResultV1) -> bool {
    let expected_paths = BTreeSet::from([
        "fixtures/routine-production-catalog/cases.json",
        "validator/src/routine_work/catalog/mod.rs",
        "validator/tests/routine_production_catalog_contract.rs",
    ]);
    let actual_paths = result
        .artifacts
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    let declared_count = result.candidate_identity["artifact_count"].as_u64();
    let declared_bytes = result.candidate_identity["artifact_bytes"].as_u64();
    let declared_aggregate = result.candidate_identity["corrected_artifact_set_sha256"].as_str();
    let actual_bytes = result
        .artifacts
        .iter()
        .map(|row| row.byte_length)
        .sum::<u64>();
    let mut aggregate_rows = result
        .artifacts
        .iter()
        .map(|row| {
            format!(
                "{}\t{}\n",
                row.path,
                row.sha256.strip_prefix("sha256:").unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    aggregate_rows.sort();
    let actual_aggregate = sha(aggregate_rows.concat().as_bytes());

    actual_paths == expected_paths
        && declared_count == Some(3)
        && result.artifacts.len() == 3
        && declared_bytes == Some(actual_bytes)
        && declared_aggregate == Some(actual_aggregate.as_str())
}

pub(crate) fn corrective_lease() -> (WorkPackage, LeaseSpec, ScopePolicy) {
    let owned_paths = canonical_paths(&[
        R3_RESULT_PATH,
        "validator/src/routine_work/catalog/mod.rs",
        "validator/tests/routine_production_catalog_contract.rs",
    ]);
    let fixtures = canonical_paths(&["fixtures/routine-production-catalog/cases.json"]);
    let semantic_symbols = BTreeSet::from(["routine-work::production-catalog".to_owned()]);
    let effects = BTreeSet::from([
        EffectGrant::new(
            EffectClass::WorkspaceWrite,
            "routine-production-catalog-source",
        )
        .unwrap(),
        EffectGrant::new(
            EffectClass::FixtureWrite,
            "routine-production-catalog-fixtures",
        )
        .unwrap(),
        EffectGrant::new(EffectClass::Process, "offline-validation").unwrap(),
    ]);
    let owned_scope = OwnedScope {
        paths: owned_paths.clone(),
        semantic_symbols: semantic_symbols.clone(),
        generated_outputs: BTreeSet::new(),
        fixtures: fixtures.clone(),
        effects: effects.clone(),
    };
    let dependencies = BTreeSet::from([
        "accepted-routine-public-execution-mediator".to_owned(),
        "generated-impact-node-catalog-authority".to_owned(),
    ]);
    let package = WorkPackage {
        node_id: "routine-production-catalog".to_owned(),
        dependencies,
        required_tools: BTreeSet::from([
            "cargo-nextest".to_owned(),
            "rustfmt".to_owned(),
            "shasum".to_owned(),
        ]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope: owned_scope.clone(),
        prerequisites: BTreeSet::from([
            "preserve-prior-r2-candidate".to_owned(),
            "root-retains-public-dispatch-and-effect-grant-authority".to_owned(),
        ]),
        outputs: BTreeSet::from([
            "catalog-bound-symbolic-runner-authority".to_owned(),
            "typed-worker-result-receipt".to_owned(),
        ]),
        acceptance: BTreeSet::from([
            "caller-consistent-executable-substitution-refused".to_owned(),
            "worker-result-artifacts-verified".to_owned(),
        ]),
        claim_effect: "none".to_owned(),
    };
    let lease = LeaseSpec {
        lease_id: "ROUTINE-PRODUCTION-CATALOG-073-R3".to_owned(),
        run_id: "ultragoal-successor-live-20260713-routine-production-catalog-r3".to_owned(),
        node_id: package.node_id.clone(),
        principal: Principal::Worker,
        owner: Actor::parse("/root/routine_catalog_single_spelling_engineer").unwrap(),
        binding: Binding::new(R3_CONTEXT_ID, R3_CANDIDATE_ID).unwrap(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope,
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 1_000_000,
        max_retries: 2,
    };
    let policy = ScopePolicy {
        allowed_paths: owned_paths,
        allowed_semantic_prefixes: semantic_symbols,
        allowed_fixtures: fixtures,
        allowed_effects: effects,
        ..ScopePolicy::default()
    };
    (package, lease, policy)
}

pub(crate) fn canonical_paths(values: &[&str]) -> BTreeSet<CanonicalPath> {
    values
        .iter()
        .map(|value| CanonicalPath::parse(value).unwrap())
        .collect()
}
