use super::super::CapturedTaskObservation;
use super::super::{
    BehaviorOutcome, BoundInput, EvaluationError, EvaluationTask, PerturbationControl, digest,
    valid_identifier, valid_sha256,
};
use super::AuthorizedEvidenceBinding;
use crate::fixture_scheduler::FixtureExecutionRecord;
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ScorerPolicy {
    schema_version: String,
    dataset_digest_sha256: String,
    executable_digest_sha256: String,
    primary_score_possible: u64,
    work_units: u64,
    failure_causal_code: String,
    perturbation_controls: BTreeSet<PerturbationControl>,
    task_authority_id: String,
    task_author_principal_id: String,
    task_author_session_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndependentGraderPolicy {
    schema_version: String,
    dataset_digest_sha256: String,
    scorer_digest_sha256: String,
    root_authority_id: String,
    independent_grader_principal_id: String,
    independent_grader_session_id: String,
    expected_artifact_text: String,
    score_possible: u64,
}

pub(crate) struct AuthenticatedTaskMaterial {
    task_id: String,
    fixture_id: String,
    dataset_digest_sha256: String,
    scorer_policy_digest_sha256: String,
    executable_digest_sha256: String,
    scorer: ScorerPolicy,
    grader: IndependentGraderPolicy,
    observer_id: String,
    independent_grader_id: String,
}

impl AuthenticatedTaskMaterial {
    pub(super) fn issue(
        task: &EvaluationTask,
        protected: &super::ProtectedTaskInputs,
        authority: &AuthorizedEvidenceBinding,
    ) -> Result<Self, EvaluationError> {
        let dataset = protected.dataset.read_authenticated()?;
        if digest(&dataset) != task.dataset.digest_sha256 {
            return Err(EvaluationError::new(
                "evaluation-task-material-binding-invalid",
            ));
        }
        let scorer_bytes = protected.scorer.read_authenticated()?;
        if digest(&scorer_bytes) != task.scorer_digest_sha256 {
            return Err(EvaluationError::new(
                "evaluation-task-material-binding-invalid",
            ));
        }
        let scorer: ScorerPolicy = serde_json::from_slice(&scorer_bytes)
            .map_err(|_| EvaluationError::new("evaluation-scorer-policy-invalid"))?;
        let scorer_policy_digest_sha256 = digest(&scorer_bytes);
        if scorer.schema_version != "EvaluationScorer-v2"
            || scorer.dataset_digest_sha256 != task.dataset.digest_sha256
            || !valid_sha256(&scorer.executable_digest_sha256)
            || scorer.primary_score_possible == 0
            || scorer.work_units == 0
            || !valid_identifier(&scorer.failure_causal_code)
            || scorer.failure_causal_code == "behavioral-pass"
            || scorer.perturbation_controls != task.perturbation_controls
            || scorer.task_authority_id != authority.task_authority_id
            || scorer.task_author_principal_id != authority.task_principal_id
            || scorer.task_author_session_id != authority.task_session_id
        {
            return Err(EvaluationError::new("evaluation-scorer-policy-invalid"));
        }
        let grader_bytes = protected.grader.read_authenticated()?;
        let grader: IndependentGraderPolicy = serde_json::from_slice(&grader_bytes)
            .map_err(|_| EvaluationError::new("evaluation-independent-grader-policy-invalid"))?;
        if grader.schema_version != "IndependentArtifactTextGrader-v2"
            || grader.dataset_digest_sha256 != task.dataset.digest_sha256
            || grader.scorer_digest_sha256 != scorer_policy_digest_sha256
            || grader.root_authority_id != authority.grader_authority_id
            || grader.independent_grader_principal_id != authority.grader_principal_id
            || grader.independent_grader_session_id != authority.grader_session_id
            || grader.expected_artifact_text.is_empty()
            || grader.expected_artifact_text.len() > 4096
            || grader.expected_artifact_text.chars().any(char::is_control)
            || grader.score_possible == 0
        {
            return Err(EvaluationError::new(
                "evaluation-independent-grader-policy-invalid",
            ));
        }
        let observer_id = derived_id(
            "provenance-observer",
            &format!(
                "{}|{}",
                authority.provenance_principal_id, authority.provenance_session_id
            ),
        );
        let independent_grader_id = derived_id(
            "independent-grader",
            &format!(
                "{}|{}",
                authority.grader_principal_id, authority.grader_session_id
            ),
        );
        Ok(Self {
            task_id: task.task_id.clone(),
            fixture_id: task.fixture_id.clone(),
            dataset_digest_sha256: task.dataset.digest_sha256.clone(),
            scorer_policy_digest_sha256,
            executable_digest_sha256: scorer.executable_digest_sha256.clone(),
            scorer,
            grader,
            observer_id,
            independent_grader_id,
        })
    }

    pub(crate) fn dataset_digest_sha256(&self) -> &str {
        &self.dataset_digest_sha256
    }
    pub(crate) fn scorer_policy_digest_sha256(&self) -> &str {
        &self.scorer_policy_digest_sha256
    }
    pub(crate) fn executable_digest_sha256(&self) -> &str {
        &self.executable_digest_sha256
    }

    pub(crate) fn score(
        self,
        record: &FixtureExecutionRecord,
    ) -> Result<CapturedTaskObservation, EvaluationError> {
        if record.binding.task_id != self.task_id
            || record.fixture_id != self.fixture_id
            || record.executable_digest_sha256 != self.executable_digest_sha256
        {
            return Err(EvaluationError::new(
                "evaluation-fixture-record-binding-invalid",
            ));
        }
        let primary_passed = record.exit_code == Some(0)
            && record.outcome.verdict == crate::fixture_scheduler::OutcomeVerdict::Pass;
        let independent_passed = std::str::from_utf8(record.artifact_bytes())
            .is_ok_and(|text| text == self.grader.expected_artifact_text);
        let passed = primary_passed && independent_passed;
        Ok(CapturedTaskObservation::captured(
            super::super::CapturedTaskObservationRecord {
                task_id: self.task_id,
                fixture_id: self.fixture_id,
                outcome: if passed {
                    BehaviorOutcome::Passed
                } else {
                    BehaviorOutcome::Failed
                },
                causal_code: if passed {
                    "behavioral-pass".to_owned()
                } else {
                    self.scorer.failure_causal_code
                },
                score_earned: u64::from(primary_passed) * self.scorer.primary_score_possible,
                score_possible: self.scorer.primary_score_possible,
                work_units: self.scorer.work_units,
                artifact: BoundInput::regular(
                    format!("artifacts/{}", record.artifact_relative_path),
                    record.artifact_digest_sha256.clone(),
                    record.artifact_byte_length,
                ),
                replay_artifact_digest_sha256: record.artifact_digest_sha256.clone(),
                producer_id: "fixture-producer".to_owned(),
                observer_id: self.observer_id,
                independent_grader_id: self.independent_grader_id,
                independent_score_earned: u64::from(independent_passed)
                    * self.grader.score_possible,
                independent_score_possible: self.grader.score_possible,
                passed_perturbations: self.scorer.perturbation_controls,
            },
        ))
    }
}

fn derived_id(label: &str, material: &str) -> String {
    format!("{}-{}", label, &digest(material.as_bytes())[7..23])
}
