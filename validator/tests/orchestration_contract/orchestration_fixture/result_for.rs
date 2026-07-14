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
