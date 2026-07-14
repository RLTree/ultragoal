static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);
pub const SECRET_CANARY: &[u8] = b"tree-root-secret-canary-0123456789abcdef";

pub fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn binding() -> Binding {
    Binding::new(&digest('a'), &digest('b')).unwrap()
}

pub fn root_actor() -> Actor {
    Actor::parse("ultra-root").unwrap()
}

pub fn worker_actor() -> Actor {
    Actor::parse("worker-a").unwrap()
}

pub fn path(value: &str) -> CanonicalPath {
    CanonicalPath::parse(value).unwrap()
}

pub fn effect(class: EffectClass, target: &str) -> EffectGrant {
    EffectGrant::new(class, target).unwrap()
}

pub fn scope() -> OwnedScope {
    OwnedScope {
        paths: BTreeSet::from([path("work/node-a")]),
        semantic_symbols: BTreeSet::from(["orchestration::product::node-a".to_owned()]),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: BTreeSet::from([effect(EffectClass::Process, "worker-effect")]),
    }
}

pub fn package() -> WorkPackage {
    WorkPackage {
        node_id: "node-a".to_owned(),
        dependencies: BTreeSet::new(),
        required_tools: BTreeSet::from(["hct-context".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([path("docs/contract")]),
        owned_scope: scope(),
        prerequisites: BTreeSet::from(["current-context".to_owned()]),
        outputs: BTreeSet::from(["node-a-candidate".to_owned()]),
        acceptance: BTreeSet::from(["independent-review".to_owned()]),
        claim_effect: "none".to_owned(),
    }
}

pub fn graph() -> WorkGraph {
    WorkGraph::derive(vec![package()]).unwrap()
}

pub fn policy() -> ScopePolicy {
    ScopePolicy {
        allowed_read_paths: BTreeSet::from([path("docs")]),
        allowed_paths: BTreeSet::from([path("work")]),
        allowed_semantic_prefixes: BTreeSet::from(["orchestration::product".to_owned()]),
        allowed_effects: BTreeSet::from([effect(EffectClass::Process, "worker-effect")]),
        ..ScopePolicy::default()
    }
}

pub fn bootstrap() -> BootstrapEvidence {
    BootstrapEvidence {
        observed_tick: 0,
        completed_nodes: BTreeMap::new(),
        available_tools: BTreeMap::from([("hct-context".to_owned(), digest('9'))]),
        satisfied_prerequisites: BTreeMap::from([("current-context".to_owned(), digest('8'))]),
    }
}

pub fn context() -> ProductContext {
    ProductContext::new(graph(), policy(), binding(), root_actor())
}

pub fn lease(deadline: u64) -> LeaseSpec {
    LeaseSpec {
        lease_id: "lease-001".to_owned(),
        run_id: "run-001".to_owned(),
        node_id: "node-a".to_owned(),
        principal: Principal::Worker,
        owner: worker_actor(),
        binding: binding(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([path("docs/contract")]),
        owned_scope: scope(),
        prerequisite_evidence: PrerequisiteEvidence {
            dependency_nodes: BTreeMap::new(),
            required_tools: bootstrap().available_tools,
            prerequisites: bootstrap().satisfied_prerequisites,
        },
        issued_tick: 1,
        heartbeat_deadline_tick: deadline,
        max_retries: 2,
    }
}

#[derive(Clone, Debug)]
pub struct TestSink {
    pub fail: bool,
}

impl EffectSink for TestSink {
    fn apply(&mut self, request: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        if self.fail {
            return Err(OrchestrationError::JournalIo);
        }
        Ok(EffectReceipt {
            operation_id: request.operation_id.clone(),
            effect: request.effect.clone(),
            receipt_digest: digest('d'),
        })
    }
}

pub struct TestRoot(PathBuf);

impl TestRoot {
    pub fn new(label: &str) -> Self {
        let serial = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "orchestration-product-{label}-{}-{serial}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn durable_engine(label: &str, fail: bool) -> (TestRoot, Orchestrator<TestSink>) {
    let root = TestRoot::new(label);
    let engine = Orchestrator::new_durable(
        graph(),
        policy(),
        binding(),
        root_actor(),
        bootstrap(),
        root.path(),
        TestSink { fail },
    )
    .unwrap();
    (root, engine)
}

pub fn authority() -> RootAuthority {
    root_authority_for_test(root_actor(), SECRET_CANARY).unwrap()
}

pub fn permit(
    authority: &RootAuthority,
    operation: RootOperation,
    workspace: &ProductWorkspace,
    head: &JournalHead,
    tick: u64,
    target: PermitTarget,
) -> RootPermit {
    issue_action_permit_for_test(
        authority,
        RootActionPermitIssuance {
            operation,
            binding: binding(),
            workspace_identity: workspace.identity(),
            journal_head_identity: &journal_head_identity(head).unwrap(),
            issued_tick: tick.saturating_sub(1),
            expires_tick: tick + 10,
            nonce: b"unique-test-nonce-0123456789",
            target,
        },
    )
    .unwrap()
}
