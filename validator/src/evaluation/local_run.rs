impl EvaluationRun {
    #[cfg(test)]
    pub(crate) fn execute_local<E: EvaluationExecutor>(
        spec: &EvaluationSpec,
        audit: &TaskAudit,
        executor: &mut E,
    ) -> Result<Self, EvaluationError> {
        let (context, candidate) = executor.binding();
        let context = context.to_owned();
        let candidate = candidate.to_owned();
        let execution_session_id = executor.session_id().to_owned();
        if !valid_sha256(&execution_session_id) {
            return Err(EvaluationError::new("evaluation-session-invalid"));
        }
        if !audit.revalidate(spec, &context, &candidate) {
            return Err(EvaluationError::new("evaluation-audit-stale-or-ineligible"));
        }
        let frozen_spec = spec.spec_sha256.clone();
        let mut results = Vec::with_capacity(spec.tasks.len());
        for task in &spec.tasks {
            let observed = executor.execute(task)?;
            if executor.binding() != (context.as_str(), candidate.as_str())
                || executor.session_id() != execution_session_id
                || spec.spec_sha256 != frozen_spec
            {
                return Err(EvaluationError::new("evaluation-input-drift"));
            }
            let mut invalid = observed.artifact.findings("evaluation-result-artifact");
            if observed.task_id != task.task_id
                || observed.fixture_id != task.fixture_id
                || !valid_identifier(&observed.causal_code)
                || observed.score_possible == 0
                || observed.score_earned > observed.score_possible
                || observed.work_units == 0
                || !valid_identifier(&observed.producer_id)
                || !valid_identifier(&observed.observer_id)
                || !valid_identifier(&observed.independent_grader_id)
                || observed.producer_id == observed.observer_id
                || observed.observer_id == task.scorer_id
                || observed.independent_grader_id == task.scorer_id
                || observed.independent_grader_id == observed.producer_id
                || observed.independent_grader_id == observed.observer_id
                || observed.independent_score_possible == 0
                || observed.independent_score_earned > observed.independent_score_possible
            {
                invalid.push("evaluation-observation-invalid".to_owned());
            }
            if observed.artifact.digest_sha256() != observed.replay_artifact_digest_sha256 {
                invalid.push("evaluation-nondeterministic-replay".to_owned());
            }
            if u128::from(observed.score_earned) * u128::from(observed.independent_score_possible)
                != u128::from(observed.independent_score_earned)
                    * u128::from(observed.score_possible)
            {
                invalid.push("evaluation-grader-disagreement".to_owned());
            }
            if observed.outcome == BehaviorOutcome::Passed
                && observed.causal_code != "behavioral-pass"
            {
                invalid.push("evaluation-pass-causal-code-invalid".to_owned());
            }
            if observed.outcome != BehaviorOutcome::Passed
                && observed.causal_code == "behavioral-pass"
            {
                invalid.push("evaluation-failure-causal-code-invalid".to_owned());
            }
            for control in &task.perturbation_controls {
                if !observed.passed_perturbations.contains(control) {
                    invalid.push(format!(
                        "evaluation-perturbation-not-observed:{}",
                        control.label()
                    ));
                }
            }
            if !invalid.is_empty() {
                return Err(EvaluationError::new("evaluation-observation-refused"));
            }
            results.push(EvaluationTaskResult {
                task_id: observed.task_id,
                fixture_id: observed.fixture_id,
                representative: task.representative,
                outcome: observed.outcome,
                causal_code: observed.causal_code,
                score_earned: observed.score_earned,
                score_possible: observed.score_possible,
                work_units: observed.work_units,
                artifact_digest_sha256: observed.artifact.digest_sha256,
                producer_id: observed.producer_id,
                observer_id: observed.observer_id,
                scorer_id: task.scorer_id.clone(),
                independent_grader_id: observed.independent_grader_id,
                independent_score_earned: observed.independent_score_earned,
                independent_score_possible: observed.independent_score_possible,
                passed_perturbations: observed.passed_perturbations,
            });
        }
        if executor.binding() != (context.as_str(), candidate.as_str())
            || executor.session_id() != execution_session_id
            || !audit.revalidate(spec, &context, &candidate)
        {
            return Err(EvaluationError::new("evaluation-final-revalidation-failed"));
        }
        results.sort_by(|left, right| left.task_id.cmp(&right.task_id));
        let run_sha256 = run_digest(spec, &execution_session_id, &results);
        Ok(Self {
            live_context_id: context,
            candidate_id: candidate,
            spec_id: spec.spec_id.clone(),
            spec_sha256: frozen_spec,
            task_set_sha256: spec.task_set_sha256.clone(),
            execution_session_id,
            run_sha256,
            results,
        })
    }

    pub fn harvest_failures(&self) -> Vec<FailureCase> {
        self.results
            .iter()
            .filter(|result| result.outcome != BehaviorOutcome::Passed)
            .map(|result| FailureCase {
                live_context_id: self.live_context_id.clone(),
                candidate_id: self.candidate_id.clone(),
                run_sha256: self.run_sha256.clone(),
                task_id: result.task_id.clone(),
                fixture_id: result.fixture_id.clone(),
                causal_code: result.causal_code.clone(),
                artifact_digest_sha256: result.artifact_digest_sha256.clone(),
                independent_observer_id: result.observer_id.clone(),
            })
            .collect()
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn run_sha256(&self) -> &str {
        &self.run_sha256
    }

    pub fn results(&self) -> &[EvaluationTaskResult] {
        &self.results
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FailureCase {
    pub live_context_id: String,
    pub candidate_id: String,
    pub run_sha256: String,
    pub task_id: String,
    pub fixture_id: String,
    pub causal_code: String,
    pub artifact_digest_sha256: String,
    pub independent_observer_id: String,
}

/// Opaque evidence that a crate-controlled review authority issued an
/// independent, candidate-bound review. There is intentionally no public
/// constructor or caller-supplied independence flag.
#[derive(Eq, PartialEq)]
pub struct PromotionReview {
    reviewer_id: String,
    authority_id: String,
    issuance_session_id: String,
    live_context_id: String,
    baseline_candidate_id: String,
    candidate_id: String,
    baseline_run_sha256: String,
    candidate_run_sha256: String,
    baseline_spec_id: String,
    candidate_spec_id: String,
    baseline_spec_sha256: String,
    candidate_spec_sha256: String,
    task_set_sha256: String,
    baseline_execution_session_id: String,
    candidate_execution_session_id: String,
    representative_journey: BoundInput,
    rollback_evidence: BoundInput,
    reviewed_artifacts: Vec<BoundInput>,
    binding_sha256: String,
    review_id: String,
    attestation_sha256: String,
}
