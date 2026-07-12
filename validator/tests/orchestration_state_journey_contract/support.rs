use crate::orchestration::product::*;
use crate::orchestration::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);
pub const ROOT_SECRET: &[u8] = b"journey-root-secret-0123456789abcdef";

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

pub fn effect() -> EffectGrant {
    EffectGrant::new(EffectClass::Process, "worker-effect").unwrap()
}

pub fn scope() -> OwnedScope {
    OwnedScope {
        paths: BTreeSet::from([path("work/node-a")]),
        semantic_symbols: BTreeSet::from(["orchestration::journey::node-a".to_owned()]),
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
        read_paths: BTreeSet::from([path("docs/contract")]),
        owned_scope: scope(),
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
        allowed_read_paths: BTreeSet::from([path("docs")]),
        allowed_paths: BTreeSet::from([path("work")]),
        allowed_semantic_prefixes: BTreeSet::from(["orchestration::journey".to_owned()]),
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
            "orchestration-state-{label}-{}-{serial}",
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
    action: &command::RootActionRequest,
    tick: u64,
) -> (RootAuthority, RootPermit) {
    let authority = authority();
    let permit = issue_permit_for_test(
        &authority,
        action.operation,
        action.authority_binding.clone(),
        &action.workspace_identity,
        &action.journal_head_identity,
        tick.saturating_sub(1),
        tick + 10,
        b"journey-unique-nonce-0123456789",
        action.target.clone(),
    )
    .unwrap();
    (authority, permit)
}

pub fn state_request(head: JournalHead, tick: u64) -> command::OrchestrationStateRequest {
    command::OrchestrationStateRequest {
        expected_head: head,
        tick,
        live_workers: BTreeSet::from(["worker-a".to_owned()]),
    }
}

pub fn recursive_fingerprint(root: &Path) -> Vec<(String, String)> {
    fn walk(base: &Path, current: &Path, rows: &mut Vec<(String, String)>) {
        let mut paths = fs::read_dir(current)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
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
        touched_semantics: vec!["orchestration::journey::node-a".to_owned()],
        generated_outputs: vec![],
        fixtures: vec![],
        effects: vec![],
        requirements: vec!["REQ-ORCH-005".to_owned()],
        dependency_nodes: vec![],
        changes: vec![BTreeMap::from([("path".to_owned(), json!(relative))])],
        commands_and_tests: vec![BTreeMap::from([
            ("command".to_owned(), json!("cargo nextest run focused")),
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
        limitations: vec!["root-reconciliation-pending".to_owned()],
        no_claim_statement: "This worker does not claim readiness, release, or completion."
            .to_owned(),
    }
}
