const HISTORICAL_RESULT: &str = "docs/ultragoal-successor-live/worker-results/FIXTURE-DETACHED-DESCENDANT-CONFINEMENT-CORRECTION-009.json";
const DECISION: &str = "docs/ultragoal-successor-live/root-decisions/FIXTURE-SCHEDULER-DETACHED-DESCENDANT-REWORK.json";
const LEASE_ID: &str = "FIXTURE-DETACHED-DESCENDANT-CONFINEMENT-CORRECTION-009";
const WORKER: &str = "/root/detached_descendant_confinement_builder";
const CURRENT_RESULT: &str = "docs/ultragoal-successor-live/worker-results/FIXTURE-SCHEDULER-PLATFORM-CONTRACT-CORRECTION-046.json";
const CURRENT_TASK: &str = "docs/ultragoal-successor-live/task-envelopes/FIXTURE-SCHEDULER-PLATFORM-CONTRACT-CORRECTION-046.json";
const CURRENT_WORK: &str = "docs/ultragoal-successor-live/work-packages/FIXTURE-SCHEDULER-PLATFORM-CONTRACT-CORRECTION-046.json";

fn path(value: &str) -> CanonicalPath {
    CanonicalPath::parse(value).unwrap()
}

fn source_paths() -> Vec<String> {
    [
        "validator/src/cli/capture/fixture/execute.rs",
        "validator/src/fixture_scheduler/confinement/backend.rs",
        "validator/src/fixture_scheduler/confinement/policy.rs",
        "validator/tests/fixture_scheduler_contract/confinement.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/detached_descendant.rs",
        "validator/tests/fixture_scheduler_contract/execution_adapter/process_group.rs",
        "validator/tests/fixture_scheduler_contract/helpers/confinement_probe.rs",
        "validator/tests/fixture_scheduler_contract/worker_result.rs",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn historical_subject(context: &str, candidate: &str) -> (LeaseSpec, WorkPackage) {
    let paths = source_paths();
    let owned = OwnedScope {
        paths: paths
            .iter()
            .map(|item| path(item))
            .chain(std::iter::once(path(HISTORICAL_RESULT)))
            .collect(),
        semantic_symbols: BTreeSet::from([
            "fixture_scheduler::detached_descendant_confinement".to_owned()
        ]),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: BTreeSet::from([
            EffectGrant::new(EffectClass::WorkspaceWrite, "fixture-detached-source").unwrap(),
            EffectGrant::new(EffectClass::FixtureWrite, "fixture-detached-tests").unwrap(),
            EffectGrant::new(EffectClass::Process, "fixture-detached-validation").unwrap(),
        ]),
    };
    let read_paths = BTreeSet::from([path(DECISION)]);
    let binding = Binding::new(context, candidate).unwrap();
    let lease = LeaseSpec {
        lease_id: LEASE_ID.to_owned(),
        run_id: "fixture-detached-descendant-confinement-009".to_owned(),
        node_id: "fixture-detached-descendant-confinement".to_owned(),
        principal: Principal::Worker,
        owner: Actor::parse(WORKER).unwrap(),
        binding,
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: read_paths.clone(),
        owned_scope: owned.clone(),
        prerequisite_evidence: PrerequisiteEvidence::default(),
        issued_tick: 1,
        heartbeat_deadline_tick: 2,
        max_retries: 0,
    };
    let package = WorkPackage {
        node_id: lease.node_id.clone(),
        dependencies: BTreeSet::from(["fixture-scheduler-detached-descendant-rework".to_owned()]),
        required_tools: BTreeSet::from(["cargo".to_owned(), "rustfmt".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths,
        owned_scope: owned,
        prerequisites: BTreeSet::new(),
        outputs: BTreeSet::from(["fixture-detached-confinement-candidate".to_owned()]),
        acceptance: BTreeSet::from(["fixture-detached-confinement-009".to_owned()]),
        claim_effect: "private-darwin-fixture-confinement-only".to_owned(),
    };
    (lease, package)
}

fn historical_inputs() -> (PathBuf, Vec<u8>, LeaseSpec, WorkPackage) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let decision_bytes = fs::read(root.join(DECISION)).unwrap();
    let decision: Value = serde_json::from_slice(&decision_bytes).unwrap();
    let context = format!("sha256:{:x}", Sha256::digest(&decision_bytes));
    let candidate = format!(
        "sha256:{}",
        decision
            .pointer("/reviewed_candidate/artifact_set_sha256")
            .and_then(Value::as_str)
            .unwrap()
    );
    let result_bytes = fs::read(root.join(HISTORICAL_RESULT)).unwrap();
    let (lease, package) = historical_subject(&context, &candidate);
    (root, result_bytes, lease, package)
}

fn current_paths() -> Vec<String> {
    [
        CURRENT_TASK,
        "validator/tests/fixture_scheduler_contract/execution_adapter.rs",
        "validator/tests/fixture_scheduler_contract/isolation.rs",
        "validator/tests/fixture_scheduler_contract/worker_result.rs",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IssuedCurrentSubject {
    lease: LeaseSpec,
    package: WorkPackage,
    work_envelope_sha256: String,
}

fn current_policy(subject: &IssuedCurrentSubject) -> ScopePolicy {
    ScopePolicy {
        allowed_read_paths: subject.lease.read_paths.clone(),
        allowed_paths: subject.lease.owned_scope.paths.clone(),
        allowed_semantic_prefixes: subject.lease.owned_scope.semantic_symbols.clone(),
        allowed_generated_outputs: subject.lease.owned_scope.generated_outputs.clone(),
        allowed_fixtures: subject.lease.owned_scope.fixtures.clone(),
        allowed_effects: subject.lease.owned_scope.effects.clone(),
        ..ScopePolicy::default()
    }
}

fn issued_current_subject(root: &PathBuf) -> IssuedCurrentSubject {
    let work_bytes = fs::read(root.join(CURRENT_WORK)).unwrap();
    let envelope: Value = serde_json::from_slice(&work_bytes).unwrap();
    assert_eq!(
        envelope.get("schema_version").and_then(Value::as_str),
        Some("RootIssuedWorkEnvelope-v1")
    );
    let lease: LeaseSpec = serde_json::from_value(envelope.get("lease").unwrap().clone()).unwrap();
    let package: WorkPackage =
        serde_json::from_value(envelope.get("work_package").unwrap().clone()).unwrap();
    let issued = envelope.get("issued_from_live_context").unwrap();
    assert_eq!(
        issued.get("context_id").and_then(Value::as_str),
        Some(lease.binding.context_id.as_str())
    );
    assert_eq!(
        issued.get("candidate_id").and_then(Value::as_str),
        Some(lease.binding.candidate_id.as_str())
    );
    assert_eq!(lease.node_id, package.node_id);
    assert_eq!(lease.safety_class, package.safety_class);
    assert_eq!(lease.read_paths, package.read_paths);
    assert_eq!(lease.owned_scope, package.owned_scope);
    assert_eq!(
        lease
            .prerequisite_evidence
            .dependency_nodes
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        package.dependencies
    );
    assert_eq!(
        lease
            .prerequisite_evidence
            .required_tools
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        package.required_tools
    );
    assert!(package.prerequisites.iter().all(|prerequisite| {
        lease
            .prerequisite_evidence
            .prerequisites
            .contains_key(prerequisite)
    }));

    let subject = IssuedCurrentSubject {
        lease,
        package,
        work_envelope_sha256: digest(&work_bytes),
    };
    let policy = current_policy(&subject);
    policy.validate().unwrap();
    subject.lease.validate(&policy).unwrap();
    subject.package.validate().unwrap();
    subject
}
