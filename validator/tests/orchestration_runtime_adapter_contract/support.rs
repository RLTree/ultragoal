use crate::orchestration::product::command::{
    InterruptedRecoveryRequest, OrchestrationStateRequest, RootActionRequest,
};
use crate::orchestration::product::*;
use crate::orchestration::*;
use crate::runtime_adapter::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);
static NEXT_NONCE: AtomicU64 = AtomicU64::new(1);
pub const ROOT_SECRET: &[u8] = b"runtime-adapter-root-secret-0123456789abcdef";

pub fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn binding() -> Binding {
    Binding::new(&digest('a'), &digest('b')).unwrap()
}

pub fn alternate_binding() -> Binding {
    Binding::new(&digest('c'), &digest('d')).unwrap()
}

pub fn root_actor() -> Actor {
    Actor::parse("ultra-root").unwrap()
}

pub fn worker_actor() -> Actor {
    Actor::parse("worker-a").unwrap()
}

pub fn canonical_path(value: &str) -> CanonicalPath {
    CanonicalPath::parse(value).unwrap()
}

pub fn effect() -> EffectGrant {
    EffectGrant::new(EffectClass::Process, "worker-effect").unwrap()
}

pub fn owned_scope() -> OwnedScope {
    OwnedScope {
        paths: BTreeSet::from([canonical_path("work/node-a")]),
        semantic_symbols: BTreeSet::from(["orchestration::runtime-adapter".to_owned()]),
        generated_outputs: BTreeSet::new(),
        fixtures: BTreeSet::new(),
        effects: BTreeSet::from([effect()]),
    }
}

pub fn package() -> WorkPackage {
    WorkPackage {
        node_id: "node-a".to_owned(),
        dependencies: BTreeSet::new(),
        required_tools: BTreeSet::from(["hct-state".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([canonical_path("docs/contract")]),
        owned_scope: owned_scope(),
        prerequisites: BTreeSet::from(["current-candidate".to_owned()]),
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
        allowed_read_paths: BTreeSet::from([canonical_path("docs")]),
        allowed_paths: BTreeSet::from([canonical_path("work")]),
        allowed_semantic_prefixes: BTreeSet::from(["orchestration".to_owned()]),
        allowed_effects: BTreeSet::from([effect()]),
        ..ScopePolicy::default()
    }
}

pub fn bootstrap() -> BootstrapEvidence {
    BootstrapEvidence {
        observed_tick: 0,
        completed_nodes: BTreeMap::new(),
        available_tools: BTreeMap::from([("hct-state".to_owned(), digest('9'))]),
        satisfied_prerequisites: BTreeMap::from([("current-candidate".to_owned(), digest('8'))]),
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
        read_paths: BTreeSet::from([canonical_path("docs/contract")]),
        owned_scope: owned_scope(),
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
            "orchestration-runtime-adapter-{label}-{}-{serial}",
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
    root_authority_for_test(root_actor(), ROOT_SECRET).unwrap()
}

pub fn permit_for_action(
    authority: &RootAuthority,
    action: &RootActionRequest,
    tick: u64,
) -> RootPermit {
    let serial = NEXT_NONCE.fetch_add(1, Ordering::Relaxed);
    let nonce = format!("runtime-adapter-nonce-{serial:020}");
    issue_action_permit_for_test(
        authority,
        action.operation,
        action.authority_binding.clone(),
        &action.workspace_identity,
        &action.journal_head_identity,
        tick.saturating_sub(1),
        tick + 10,
        nonce.as_bytes(),
        action.target.clone(),
    )
    .unwrap()
}

pub fn permit_for_reconciliation(
    authority: &RootAuthority,
    action: &RootActionRequest,
    tick: u64,
    resolution: &EffectResolution,
) -> RootPermit {
    let serial = NEXT_NONCE.fetch_add(1, Ordering::Relaxed);
    let nonce = format!("runtime-reconcile-nonce-{serial:020}");
    issue_reconcile_permit_for_test(
        authority,
        action.authority_binding.clone(),
        &action.workspace_identity,
        &action.journal_head_identity,
        tick.saturating_sub(1),
        tick + 10,
        nonce.as_bytes(),
        action.target.clone(),
        resolution,
    )
    .unwrap()
}

pub fn not_applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('e'),
        outcome: EffectOutcome::NotApplied,
    }
}

pub fn applied_resolution() -> EffectResolution {
    EffectResolution {
        operation_id: "operation-001".to_owned(),
        evidence_digest: digest('f'),
        outcome: EffectOutcome::Applied {
            receipt: EffectReceipt {
                operation_id: "operation-001".to_owned(),
                effect: effect(),
                receipt_digest: digest('d'),
            },
        },
    }
}

pub fn state_request(head: JournalHead, tick: u64) -> OrchestrationStateRequest {
    OrchestrationStateRequest {
        expected_head: head,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    }
}

pub fn worker_result(artifacts: &TestRoot) -> WorkerResultV1 {
    let relative = "work/node-a/output.json";
    let bytes = b"{\"result\":\"bounded\"}\n";
    let target = artifacts.path().join(relative);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, bytes).unwrap();
    WorkerResultV1 {
        worker: "worker-a".to_owned(),
        lease_id: "lease-001".to_owned(),
        context_id: binding().context_id.clone(),
        candidate_identity: BTreeMap::from([
            (
                "candidate_id".to_owned(),
                Value::String(binding().candidate_id),
            ),
            ("context_id".to_owned(), Value::String(binding().context_id)),
        ]),
        base_state: BTreeMap::from([("status".to_owned(), json!("leased"))]),
        final_state: BTreeMap::from([(
            "status".to_owned(),
            json!("candidate_for_root_acceptance"),
        )]),
        touched_paths: vec![relative.to_owned()],
        touched_semantics: vec!["orchestration::runtime-adapter".to_owned()],
        generated_outputs: vec![],
        fixtures: vec![],
        effects: vec![],
        requirements: vec!["REQ-ORCH-005".to_owned()],
        dependency_nodes: vec![],
        changes: vec![BTreeMap::from([("path".to_owned(), json!(relative))])],
        commands_and_tests: vec![BTreeMap::from([
            (
                "command".to_owned(),
                json!("cargo nextest run --test orchestration_runtime_adapter_contract"),
            ),
            ("status".to_owned(), json!("pass")),
        ])],
        artifacts: vec![ArtifactRecord {
            path: relative.to_owned(),
            sha256: content_digest(bytes),
            byte_length: bytes.len() as u64,
        }],
        findings: vec![],
        unresolved_dependencies: vec![],
        requested_root_changes: vec![],
        limitations: vec![],
        no_claim_statement: "This worker does not claim readiness, release, or completion."
            .to_owned(),
    }
}

pub fn recursive_fingerprint(root: &Path) -> Vec<(String, String)> {
    fn walk(base: &Path, current: &Path, rows: &mut Vec<(String, String)>) {
        let mut entries = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            let relative = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.is_dir() {
                rows.push((relative, "directory".to_owned()));
                walk(base, &path, rows);
            } else if metadata.file_type().is_symlink() {
                rows.push((relative, "symlink".to_owned()));
            } else if metadata.is_file() {
                rows.push((relative, content_digest(&fs::read(&path).unwrap())));
            } else {
                rows.push((relative, "special".to_owned()));
            }
        }
    }
    let mut rows = Vec::new();
    walk(root, root, &mut rows);
    rows
}

pub fn interrupted_root(label: &str) -> (TestRoot, JournalHead, String) {
    let (root, mut engine) = durable_engine(label, false);
    let artifacts = TestRoot::new(&format!("{label}-artifacts"));
    let result = worker_result(&artifacts);
    let lease = lease(40);
    engine.grant_lease(1, lease.clone()).unwrap();
    engine.start(2, &lease.lease_id).unwrap();
    engine
        .submit(
            3,
            &lease.lease_id,
            &result,
            &ArtifactWorkspace::new(artifacts.path()).unwrap(),
        )
        .unwrap();
    let commitment = engine
        .events()
        .iter()
        .find_map(|event| match &event.event {
            EventKind::WorkerSubmitted {
                result_commitment_id,
                ..
            } => Some(result_commitment_id.clone()),
            _ => None,
        })
        .unwrap();
    engine.interrupt_root(4).unwrap();
    let head = engine.journal_head().unwrap().clone();
    (root, head, commitment)
}

pub fn ambiguous_effect(label: &str) -> (TestRoot, JournalHead) {
    let (root, mut engine) = durable_engine(label, true);
    engine.grant_lease(1, lease(40)).unwrap();
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
    let head = engine.journal_head().unwrap().clone();
    (root, head)
}

pub fn interrupted_heartbeat(
    label: &str,
) -> (
    TestRoot,
    JournalHead,
    OrchestrationEvent,
    InterruptedRecoveryRequest,
) {
    let (root, mut engine) = durable_engine(label, false);
    engine.grant_lease(1, lease(40)).unwrap();
    engine.start(2, "lease-001").unwrap();
    let prior = engine.journal_head().unwrap().clone();
    let event = engine
        .event_log()
        .next(
            &binding(),
            worker_actor(),
            3,
            EventKind::Heartbeat {
                lease_id: "lease-001".to_owned(),
            },
        )
        .unwrap();
    drop(engine);
    let mut bytes = fs::read(root.path().join("events.jsonl")).unwrap();
    bytes.extend(journal_frame_bytes(&event));
    bytes.push(b'\n');
    fs::write(root.path().join("events.jsonl"), bytes).unwrap();
    let request = InterruptedRecoveryRequest {
        expected_prior_head: prior.clone(),
        expected_event_id: event.event_id.clone(),
        recovered_binding: binding(),
        tick: 3,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    };
    (root, prior, event, request)
}

pub fn journal_frame_bytes(event: &OrchestrationEvent) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Frame<'a> {
        schema_version: &'static str,
        event: &'a OrchestrationEvent,
    }
    serde_json::to_vec(&Frame {
        schema_version: "OrchestrationJournalFrame-v1",
        event,
    })
    .unwrap()
}
