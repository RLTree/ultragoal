#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct CapturedTaskObservation {
    task_id: String,
    fixture_id: String,
    outcome: BehaviorOutcome,
    causal_code: String,
    score_earned: u64,
    score_possible: u64,
    work_units: u64,
    artifact: BoundInput,
    replay_artifact_digest_sha256: String,
    producer_id: String,
    observer_id: String,
    independent_grader_id: String,
    independent_score_earned: u64,
    independent_score_possible: u64,
    passed_perturbations: BTreeSet<PerturbationControl>,
}

#[cfg(test)]
pub(crate) struct CapturedTaskObservationRecord {
    pub task_id: String,
    pub fixture_id: String,
    pub outcome: BehaviorOutcome,
    pub causal_code: String,
    pub score_earned: u64,
    pub score_possible: u64,
    pub work_units: u64,
    pub artifact: BoundInput,
    pub replay_artifact_digest_sha256: String,
    pub producer_id: String,
    pub observer_id: String,
    pub independent_grader_id: String,
    pub independent_score_earned: u64,
    pub independent_score_possible: u64,
    pub passed_perturbations: BTreeSet<PerturbationControl>,
}

#[cfg(test)]
impl CapturedTaskObservation {
    pub(crate) fn captured(record: CapturedTaskObservationRecord) -> Self {
        let CapturedTaskObservationRecord {
            task_id,
            fixture_id,
            outcome,
            causal_code,
            score_earned,
            score_possible,
            work_units,
            artifact,
            replay_artifact_digest_sha256,
            producer_id,
            observer_id,
            independent_grader_id,
            independent_score_earned,
            independent_score_possible,
            passed_perturbations,
        } = record;
        Self {
            task_id,
            fixture_id,
            outcome,
            causal_code,
            score_earned,
            score_possible,
            work_units,
            artifact,
            replay_artifact_digest_sha256,
            producer_id,
            observer_id,
            independent_grader_id,
            independent_score_earned,
            independent_score_possible,
            passed_perturbations,
        }
    }
}

#[cfg(test)]
pub(crate) trait EvaluationExecutor {
    fn binding(&self) -> (&str, &str);
    fn session_id(&self) -> &str;
    fn execute(
        &mut self,
        task: &EvaluationTask,
    ) -> Result<CapturedTaskObservation, EvaluationError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationTaskResult {
    task_id: String,
    fixture_id: String,
    representative: bool,
    outcome: BehaviorOutcome,
    causal_code: String,
    score_earned: u64,
    score_possible: u64,
    work_units: u64,
    artifact_digest_sha256: String,
    producer_id: String,
    observer_id: String,
    scorer_id: String,
    independent_grader_id: String,
    independent_score_earned: u64,
    independent_score_possible: u64,
    passed_perturbations: BTreeSet<PerturbationControl>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationRun {
    live_context_id: String,
    candidate_id: String,
    spec_id: String,
    spec_sha256: String,
    task_set_sha256: String,
    execution_session_id: String,
    run_sha256: String,
    results: Vec<EvaluationTaskResult>,
}
