pub fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub fn binding() -> Binding {
    Binding::new(&digest('a'), &digest('b')).unwrap()
}

pub fn root_actor() -> Actor {
    Actor::parse("ultra-root").unwrap()
}

pub fn effect() -> EffectGrant {
    EffectGrant::new(EffectClass::Process, "worker-effect").unwrap()
}

fn owned_scope() -> OwnedScope {
    OwnedScope {
        paths: BTreeSet::from([CanonicalPath::parse("work/node-a").unwrap()]),
        semantic_symbols: BTreeSet::from(["orchestration::product::node-a".to_owned()]),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: BTreeSet::from([effect()]),
    }
}

pub(super) fn graph() -> WorkGraph {
    WorkGraph::derive(vec![WorkPackage {
        node_id: "node-a".to_owned(),
        dependencies: BTreeSet::new(),
        required_tools: BTreeSet::from(["hct-state".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope: owned_scope(),
        prerequisites: BTreeSet::new(),
        outputs: BTreeSet::from(["candidate".to_owned()]),
        acceptance: BTreeSet::from(["root-review".to_owned()]),
        claim_effect: "none".to_owned(),
    }])
    .unwrap()
}

pub(super) fn policy() -> ScopePolicy {
    ScopePolicy {
        allowed_paths: BTreeSet::from([CanonicalPath::parse("work").unwrap()]),
        allowed_semantic_prefixes: BTreeSet::from(["orchestration::product".to_owned()]),
        allowed_effects: BTreeSet::from([effect()]),
        ..ScopePolicy::default()
    }
}

pub fn context() -> ProductContext {
    ProductContext::new(graph(), policy(), binding(), root_actor())
}

#[derive(Clone, Debug)]
struct NoEffect;

impl EffectSink for NoEffect {
    fn apply(&mut self, _: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        Err(OrchestrationError::EffectDenied)
    }
}

fn engine(root: &TestRoot) -> Orchestrator<NoEffect> {
    Orchestrator::new_durable(
        graph(),
        policy(),
        binding(),
        root_actor(),
        BootstrapEvidence {
            observed_tick: 0,
            completed_nodes: BTreeMap::new(),
            available_tools: BTreeMap::from([("hct-state".to_owned(), digest('9'))]),
            satisfied_prerequisites: BTreeMap::new(),
        },
        root.path(),
        NoEffect,
    )
    .unwrap()
}

pub fn running_lease(label: &str, deadline: u64) -> (TestRoot, JournalHead) {
    let root = TestRoot::new(label, 0o700);
    let mut orchestrator = engine(&root);
    orchestrator.grant_lease(1, lease(deadline)).unwrap();
    orchestrator.start(2, "lease-001").unwrap();
    let head = orchestrator.journal_head().unwrap().clone();
    (root, head)
}

pub fn interrupted_root(label: &str) -> (TestRoot, JournalHead) {
    let root = TestRoot::new(label, 0o700);
    let mut engine = engine(&root);
    engine.interrupt_root(1).unwrap();
    (root, engine.journal_head().unwrap().clone())
}

pub fn advance_interrupted_root(root: &TestRoot, head: JournalHead, tick: u64) {
    let mut engine =
        Orchestrator::restart_durable(graph(), policy(), head, root_actor(), root.path(), NoEffect)
            .unwrap();
    engine.recover_root(tick).unwrap();
}

pub fn ambiguous_effect(label: &str) -> (TestRoot, JournalHead) {
    let root = TestRoot::new(label, 0o700);
    let mut engine = engine(&root);
    engine.grant_lease(1, lease(20)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let request = EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(),
        operation_id: "operation-001".to_owned(),
        payload_digest: digest('6'),
    };
    assert_eq!(
        engine.apply_effect(3, request).unwrap_err(),
        OrchestrationError::EffectAmbiguous
    );
    (root, engine.journal_head().unwrap().clone())
}

pub(super) fn lease(deadline: u64) -> LeaseSpec {
    LeaseSpec {
        lease_id: "lease-001".to_owned(),
        run_id: "run-001".to_owned(),
        node_id: "node-a".to_owned(),
        principal: Principal::Worker,
        owner: Actor::parse("worker-a").unwrap(),
        binding: binding(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::new(),
        owned_scope: owned_scope(),
        prerequisite_evidence: PrerequisiteEvidence {
            dependency_nodes: BTreeMap::new(),
            required_tools: BTreeMap::from([("hct-state".to_owned(), digest('9'))]),
            prerequisites: BTreeMap::new(),
        },
        issued_tick: 1,
        heartbeat_deadline_tick: deadline,
        max_retries: 1,
    }
}

pub fn not_applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('e'),
        outcome: EffectOutcome::NotApplied,
    }
}

pub fn interrupted_heartbeat(
    label: &str,
) -> (
    TestRoot,
    JournalHead,
    OrchestrationEvent,
    InterruptedRecoveryRequest,
) {
    let root = TestRoot::new(label, 0o700);
    let mut engine = engine(&root);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let prior_head_bytes = fs::read(root.path().join("head.json")).unwrap();
    engine.heartbeat(3, "lease-001").unwrap();
    let event = engine.events().last().unwrap().clone();
    drop(engine);
    fs::write(root.path().join("head.json"), prior_head_bytes).unwrap();
    let request = InterruptedRecoveryRequest {
        expected_prior_head: prior.clone(),
        expected_event_id: event.event_id.clone(),
        recovered_binding: binding(),
        tick: 3,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    };
    (root, prior, event, request)
}
