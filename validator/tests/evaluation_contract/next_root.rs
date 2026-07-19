fn require_type<T>() {}

#[test]
fn required_evaluation_api_types_compile_in_the_leased_module() {
    require_type::<EvaluationSpec>();
    require_type::<TaskAudit>();
    require_type::<EvaluationRun>();
    require_type::<FailureCase>();
    require_type::<PromotionDecision>();
}

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn controls() -> BTreeSet<PerturbationControl> {
    PerturbationControl::REQUIRED.into_iter().collect()
}

fn task(id: &str, representative: bool) -> EvaluationTask {
    let dataset_digest = match id {
        "core" => sha('a'),
        "recovery" => sha('0'),
        _ => sha('f'),
    };
    EvaluationTask::new(EvaluationTaskDefinition {
        task_id: id.to_owned(),
        requirement_id: format!("REQ-{id}"),
        behavior_id: format!("behavior-{id}"),
        fixture_id: format!("fixture-{id}"),
        dataset: BoundInput::regular(format!("datasets/{id}.json"), dataset_digest, 128),
        scorer_id: format!("scorer-{id}"),
        scorer_digest_sha256: sha('b'),
        perturbation_controls: controls(),
        representative,
    })
}

fn spec(candidate: char) -> EvaluationSpec {
    EvaluationSpec::new(
        sha('c'),
        sha(candidate),
        "representative-suite",
        vec![task("core", true), task("recovery", true)],
    )
    .unwrap()
}

#[derive(Clone)]
struct ScriptedExecutor {
    context: String,
    candidate: String,
    session: String,
    rows: BTreeMap<String, CapturedTaskObservation>,
    drift_after_first: bool,
    drift_session_after_first: bool,
    calls: usize,
}

impl ScriptedExecutor {
    fn new(candidate: char, core: BehaviorOutcome, core_score: u64) -> Self {
        let mut rows = BTreeMap::new();
        rows.insert(
            "core".to_owned(),
            observation("core", core, core_score, 'd'),
        );
        rows.insert(
            "recovery".to_owned(),
            observation("recovery", BehaviorOutcome::Passed, 10, 'e'),
        );
        Self {
            context: sha('c'),
            candidate: sha(candidate),
            session: execution_session(candidate),
            rows,
            drift_after_first: false,
            drift_session_after_first: false,
            calls: 0,
        }
    }
}

impl EvaluationExecutor for ScriptedExecutor {
    fn binding(&self) -> (&str, &str) {
        (&self.context, &self.candidate)
    }

    fn session_id(&self) -> &str {
        &self.session
    }

    fn execute(
        &mut self,
        task: &EvaluationTask,
    ) -> Result<CapturedTaskObservation, EvaluationError> {
        self.calls += 1;
        let row = self.rows.get(task.task_id()).unwrap().clone();
        if self.drift_after_first && self.calls == 1 {
            self.candidate = sha('f');
        }
        if self.drift_session_after_first && self.calls == 1 {
            self.session = sha('f');
        }
        Ok(row)
    }
}

fn execution_session(candidate: char) -> String {
    match candidate {
        '1' => sha('4'),
        '2' => sha('5'),
        _ => sha('6'),
    }
}

fn observation(
    id: &str,
    outcome: BehaviorOutcome,
    score: u64,
    digest_byte: char,
) -> CapturedTaskObservation {
    let causal = if outcome == BehaviorOutcome::Passed {
        "behavioral-pass"
    } else {
        "causal-product-failure"
    };
    CapturedTaskObservation::captured(CapturedTaskObservationRecord {
        task_id: id.to_owned(),
        fixture_id: format!("fixture-{id}"),
        outcome,
        causal_code: causal.to_owned(),
        score_earned: score,
        score_possible: 10,
        work_units: 3,
        artifact: BoundInput::regular(format!("artifacts/{id}.json"), sha(digest_byte), 96),
        replay_artifact_digest_sha256: sha(digest_byte),
        producer_id: format!("executor-{id}"),
        observer_id: format!("observer-{id}"),
        independent_grader_id: format!("grader-{id}"),
        independent_score_earned: score,
        independent_score_possible: 10,
        passed_perturbations: controls(),
    })
}

fn run(candidate: char, core: BehaviorOutcome, score: u64) -> EvaluationRun {
    let spec = spec(candidate);
    let audit = spec.audit(&sha('c'), &sha(candidate));
    let mut executor = ScriptedExecutor::new(candidate, core, score);
    EvaluationRun::execute_local(&spec, &audit, &mut executor).unwrap()
}

struct ReviewAuthorityHarness {
    authority: PromotionReviewAuthority,
}

impl ReviewAuthorityHarness {
    fn current(baseline: &EvaluationRun, candidate: &EvaluationRun) -> Self {
        Self::with_ids(
            baseline,
            candidate,
            "root-review-authority",
            "independent-reviewer",
            sha('7'),
        )
        .unwrap()
    }

    fn with_ids(
        baseline: &EvaluationRun,
        candidate: &EvaluationRun,
        authority_id: &str,
        reviewer_id: &str,
        review_session_id: String,
    ) -> Result<Self, &'static str> {
        let baseline_proof = execution_terminal_proof(baseline);
        let candidate_proof = execution_terminal_proof(candidate);
        let binding = PromotionLedgerBinding::from_terminal_proofs(
            authority_id,
            reviewer_id,
            review_session_id,
            baseline_proof,
            candidate_proof,
        )
        .map_err(|error| error.code())?;
        let ledger_root = private_ledger_root();
        let ledger = match FilePromotionReviewLedger::initialize(&ledger_root, [7; 32], binding) {
            Ok(ledger) => ledger,
            Err(error) => {
                let code = error.code();
                return Err(code);
            }
        };
        let authority = match ledger.bind_review_evidence(env!("CARGO_MANIFEST_DIR")) {
            Ok(authority) => authority,
            Err(error) => {
                let code = error.code();
                return Err(code);
            }
        };
        Ok(Self { authority })
    }
}

impl std::ops::Deref for ReviewAuthorityHarness {
    type Target = PromotionReviewAuthority;

    fn deref(&self) -> &Self::Target {
        &self.authority
    }
}

impl std::ops::DerefMut for ReviewAuthorityHarness {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.authority
    }
}

fn execution_terminal_proof(run: &EvaluationRun) -> super::super::ledger::ExecutionTerminalProof {
    super::super::ledger::ExecutionTerminalProof {
        binding: EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
            live_context_id: run.live_context_id.clone(),
            candidate_id: run.candidate_id.clone(),
            spec_sha256: run.spec_sha256.clone(),
            task_set_sha256: run.task_set_sha256.clone(),
            execution_session_id: run.execution_session_id.clone(),
            execution_material_set_sha256: sha('e'),
            artifact_root_sha256: sha('f'),
        })
        .unwrap(),
        run_sha256: run.run_sha256.clone(),
        artifact_set_sha256: sha('9'),
        ledger_head_sha256: sha('8'),
    }
}
