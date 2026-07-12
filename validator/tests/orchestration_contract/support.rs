use crate::orchestration::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

pub fn digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub const LIVE_LIB_BYTES: &[u8] = b"pub mod orchestration;\n";
pub const LIVE_MANIFEST_BYTES: &[u8] = b"{\"name\":\"harness-ultragoal\"}\n";
pub const LIVE_PRIOR_BYTES: &[u8] = b"prior root bytes\n";

pub fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub fn binding() -> Binding {
    Binding::new(&digest('a'), &digest('b')).expect("binding")
}

pub fn root() -> Actor {
    Actor::parse("ultra-root").expect("root actor")
}

pub fn bootstrap() -> BootstrapEvidence {
    BootstrapEvidence {
        observed_tick: 0,
        completed_nodes: BTreeMap::new(),
        available_tools: BTreeMap::from([("hct-context".to_owned(), digest('9'))]),
        satisfied_prerequisites: BTreeMap::from([("current-context".to_owned(), digest('8'))]),
    }
}

pub fn worker(name: &str) -> Actor {
    Actor::parse(name).expect("worker actor")
}

pub fn path(value: &str) -> CanonicalPath {
    CanonicalPath::parse(value).expect("canonical path")
}

pub fn effect(class: EffectClass, target: &str) -> EffectGrant {
    EffectGrant::new(class, target).expect("effect")
}

pub fn policy() -> ScopePolicy {
    ScopePolicy {
        allowed_read_paths: BTreeSet::from([path("docs/ultragoal-successor-live")]),
        allowed_paths: BTreeSet::from([
            path("validator/src/orchestration"),
            path("validator/tests/orchestration_contract"),
        ]),
        allowed_semantic_prefixes: BTreeSet::from(["orchestration".to_owned()]),
        allowed_generated_outputs: BTreeSet::from([path("generated/orchestration")]),
        allowed_fixtures: BTreeSet::from([path("validator/tests/orchestration_contract")]),
        allowed_effects: BTreeSet::from([
            effect(EffectClass::WorkspaceWrite, "leased-source"),
            effect(EffectClass::FixtureWrite, "isolated-test"),
            effect(EffectClass::Process, "focused-test"),
        ]),
        root_only_paths: BTreeSet::from([
            path("Cargo.lock"),
            path("plugin-manifest-draft.json"),
            path("migration/authority-routes.json"),
        ]),
        root_only_semantic_prefixes: BTreeSet::from([
            "root::claims".to_owned(),
            "root::public_commands".to_owned(),
        ]),
        root_only_effect_classes: BTreeSet::from([
            EffectClass::Destructive,
            EffectClass::RootAuthority,
        ]),
    }
}

pub fn scope(stem: &str) -> OwnedScope {
    OwnedScope {
        paths: BTreeSet::from([path(&format!("validator/src/orchestration/{stem}.rs"))]),
        semantic_symbols: BTreeSet::from([format!("orchestration::{stem}")]),
        generated_outputs: BTreeSet::from([path(&format!("generated/orchestration/{stem}.json"))]),
        fixtures: BTreeSet::from([path(&format!(
            "validator/tests/orchestration_contract/{stem}.json"
        ))]),
        effects: BTreeSet::from([effect(
            EffectClass::WorkspaceWrite,
            &format!("leased-source/{stem}"),
        )]),
    }
}

pub fn package(node: &str, dependencies: &[&str], stem: &str) -> WorkPackage {
    WorkPackage {
        node_id: node.to_owned(),
        dependencies: dependencies
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        required_tools: BTreeSet::from(["hct-context".to_owned()]),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([path("docs/ultragoal-successor-live")]),
        owned_scope: scope(stem),
        prerequisites: BTreeSet::from(["current-context".to_owned()]),
        outputs: BTreeSet::from([format!("{node}-candidate")]),
        acceptance: BTreeSet::from([format!("{node}-independent-review")]),
        claim_effect: "observation-only".to_owned(),
    }
}

pub fn graph_one() -> WorkGraph {
    WorkGraph::derive(vec![package("node-a", &[], "node_a")]).expect("graph")
}

pub fn lease_with_scope(
    lease_id: &str,
    node: &str,
    owner: &str,
    owned_scope: OwnedScope,
) -> LeaseSpec {
    LeaseSpec {
        lease_id: lease_id.to_owned(),
        run_id: "run-001".to_owned(),
        node_id: node.to_owned(),
        principal: Principal::Worker,
        owner: worker(owner),
        binding: binding(),
        safety_class: SafetyClass::IsolatedWorkspaceWrite,
        read_paths: BTreeSet::from([path("docs/ultragoal-successor-live")]),
        owned_scope,
        prerequisite_evidence: PrerequisiteEvidence {
            dependency_nodes: BTreeMap::new(),
            required_tools: BTreeMap::from([("hct-context".to_owned(), digest('9'))]),
            prerequisites: BTreeMap::from([("current-context".to_owned(), digest('8'))]),
        },
        issued_tick: 1,
        heartbeat_deadline_tick: 20,
        max_retries: 2,
    }
}

pub fn lease() -> LeaseSpec {
    lease_with_scope("lease-001", "node-a", "worker-a", scope("node_a"))
}

pub fn record(key: &str, value: Value) -> BTreeMap<String, Value> {
    BTreeMap::from([(key.to_owned(), value)])
}

pub fn commitment_id(result: &WorkerResultV1) -> String {
    let node = result.touched_semantics[0]
        .trim_start_matches("orchestration::")
        .replace('_', "-");
    AcceptanceProposal::result_commitment_id_for_parts(&binding(), &node, &result.lease_id, result)
        .unwrap()
}

pub fn result_for(lease: &LeaseSpec, package: &WorkPackage) -> WorkerResultV1 {
    let stem = package.node_id.replace('-', "_");
    let artifact_path = format!("validator/src/orchestration/{stem}.rs");
    let generated_path = format!("generated/orchestration/{stem}.json");
    let fixture_path = format!("validator/tests/orchestration_contract/{stem}.json");
    WorkerResultV1 {
        worker: lease.owner.as_str().to_owned(),
        lease_id: lease.lease_id.clone(),
        context_id: lease.binding.context_id.clone(),
        candidate_identity: BTreeMap::from([
            (
                "candidate_id".to_owned(),
                Value::String(lease.binding.candidate_id.clone()),
            ),
            (
                "context_id".to_owned(),
                Value::String(lease.binding.context_id.clone()),
            ),
        ]),
        base_state: record("status", json!("leased")),
        final_state: record("status", json!("candidate_for_root_acceptance")),
        touched_paths: vec![
            artifact_path.clone(),
            generated_path.clone(),
            fixture_path.clone(),
        ],
        touched_semantics: vec![format!("orchestration::{stem}")],
        generated_outputs: vec![generated_path],
        fixtures: vec![fixture_path],
        effects: vec![EffectUse {
            class: EffectClass::WorkspaceWrite,
            target: lease.owned_scope.effects.first().unwrap().target.clone(),
            performed: true,
        }],
        requirements: vec!["REQ-ORCH-004".to_owned()],
        dependency_nodes: package.dependencies.iter().cloned().collect(),
        changes: vec![record("path", json!(artifact_path))],
        commands_and_tests: vec![BTreeMap::from([
            ("command".to_owned(), json!("cargo test focused")),
            ("status".to_owned(), json!("pass")),
        ])],
        artifacts: vec![ArtifactRecord {
            path: artifact_path,
            sha256: digest('c'),
            byte_length: 42,
        }],
        findings: vec![],
        unresolved_dependencies: vec![],
        requested_root_changes: vec![
            RootChangeRequest::new(
                "validator/src/lib.rs",
                &content_digest(LIVE_LIB_BYTES),
                Some("root module wiring"),
            )
            .unwrap(),
        ],
        limitations: vec!["root-integration-pending".to_owned()],
        no_claim_statement: "This worker does not claim readiness, release, or completion."
            .to_owned(),
    }
}

pub fn review_for_result(result: &WorkerResultV1, decision: ReviewDecision) -> ReviewRecord {
    ReviewRecord {
        reviewer: "reviewer-a".to_owned(),
        worker: result.worker.clone(),
        binding: binding(),
        result_id: result.result_id().unwrap(),
        result_commitment_id: commitment_id(result),
        decision,
        reproduced_commands: BTreeSet::from(["cargo-test-focused".to_owned()]),
        finding_codes: if decision == ReviewDecision::Pass {
            BTreeSet::new()
        } else {
            BTreeSet::from(["finding-material".to_owned()])
        },
    }
}

#[derive(Clone)]
pub struct CountingSink {
    pub calls: Rc<Cell<usize>>,
    pub mismatch: bool,
    pub fail: bool,
}

impl CountingSink {
    pub fn new() -> (Self, Rc<Cell<usize>>) {
        let calls = Rc::new(Cell::new(0));
        let sink = Self {
            calls: Rc::clone(&calls),
            mismatch: false,
            fail: false,
        };
        (sink, calls)
    }
}

impl EffectSink for CountingSink {
    fn apply(&mut self, request: &EffectRequest) -> Result<EffectReceipt, OrchestrationError> {
        self.calls.set(self.calls.get() + 1);
        if self.fail {
            return Err(OrchestrationError::JournalIo);
        }
        Ok(EffectReceipt {
            operation_id: if self.mismatch {
                "wrong-operation".to_owned()
            } else {
                request.operation_id.clone()
            },
            effect: request.effect.clone(),
            receipt_digest: digest('d'),
        })
    }
}

pub fn engine() -> (Orchestrator<CountingSink>, Rc<Cell<usize>>) {
    let (sink, calls) = CountingSink::new();
    let engine = Orchestrator::new(graph_one(), policy(), binding(), root(), bootstrap(), sink)
        .expect("engine");
    (engine, calls)
}
