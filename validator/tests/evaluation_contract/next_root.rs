static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

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
        artifact: BoundInput::regular(
            format!("artifacts/{id}.json"),
            sha(digest_byte),
            96,
        ),
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

struct TestReviewAuthority {
    authority_id: String,
    reviewer_id: String,
    session_id: String,
    context_id: String,
    candidate_id: String,
    secret: String,
    consumed: BTreeSet<String>,
}

impl TestReviewAuthority {
    fn current(candidate: char) -> Self {
        Self {
            authority_id: "root-review-authority".to_owned(),
            reviewer_id: "independent-reviewer".to_owned(),
            session_id: sha('7'),
            context_id: sha('c'),
            candidate_id: sha(candidate),
            secret: "test-only-sealed-review-key".to_owned(),
            consumed: BTreeSet::new(),
        }
    }

    fn attestation(&self, binding_sha256: &str, reviewer_id: &str) -> String {
        test_digest(
            format!(
                "{}|{}|{}|{}",
                self.secret, self.authority_id, binding_sha256, reviewer_id
            )
            .as_bytes(),
        )
    }
}
