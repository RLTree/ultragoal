//! Vendor-neutral, candidate-bound evaluation and promotion decisions.
//!
//! Execution is deliberately crate-controlled. External callers may describe
//! evaluation inputs and inspect records, but they cannot implement the sealed
//! executor or mint captured task observations.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_TASKS: usize = 1_024;
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationError {
    code: &'static str,
}

impl EvaluationError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for EvaluationError {}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Regular,
    Directory,
    Symlink,
    Special,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoundInput {
    relative_path: String,
    digest_sha256: String,
    byte_length: u64,
    link_count: u64,
    kind: InputKind,
}

impl BoundInput {
    pub fn observed(
        relative_path: impl Into<String>,
        digest_sha256: impl Into<String>,
        byte_length: u64,
        link_count: u64,
        kind: InputKind,
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            digest_sha256: digest_sha256.into(),
            byte_length,
            link_count,
            kind,
        }
    }

    pub fn regular(
        relative_path: impl Into<String>,
        digest_sha256: impl Into<String>,
        byte_length: u64,
    ) -> Self {
        Self::observed(
            relative_path,
            digest_sha256,
            byte_length,
            1,
            InputKind::Regular,
        )
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    fn findings(&self, prefix: &str) -> Vec<String> {
        let mut findings = Vec::new();
        if !safe_relative_path(&self.relative_path) {
            findings.push(format!("{prefix}-unsafe-path"));
        }
        if !valid_sha256(&self.digest_sha256) {
            findings.push(format!("{prefix}-invalid-digest"));
        }
        if self.byte_length == 0 || self.byte_length > MAX_INPUT_BYTES {
            findings.push(format!("{prefix}-size-out-of-bounds"));
        }
        if self.kind != InputKind::Regular {
            findings.push(format!("{prefix}-non-regular-input"));
        }
        if self.link_count != 1 {
            findings.push(format!("{prefix}-hardlink-rejected"));
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{:?}",
            self.relative_path, self.digest_sha256, self.byte_length, self.link_count, self.kind
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PerturbationControl {
    Verbosity,
    ProofArtifact,
    ReceiptProduction,
    TestManipulation,
    ScoreOnly,
}

impl PerturbationControl {
    pub const REQUIRED: [Self; 5] = [
        Self::Verbosity,
        Self::ProofArtifact,
        Self::ReceiptProduction,
        Self::TestManipulation,
        Self::ScoreOnly,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Verbosity => "verbosity",
            Self::ProofArtifact => "proof-artifact",
            Self::ReceiptProduction => "receipt-production",
            Self::TestManipulation => "test-manipulation",
            Self::ScoreOnly => "score-only",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationTask {
    task_id: String,
    requirement_id: String,
    behavior_id: String,
    fixture_id: String,
    dataset: BoundInput,
    dataset_provenance_sha256: String,
    known_training_corpus_sha256s: BTreeSet<String>,
    scorer_id: String,
    scorer_digest_sha256: String,
    perturbation_controls: BTreeSet<PerturbationControl>,
    representative: bool,
}

impl EvaluationTask {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_id: impl Into<String>,
        requirement_id: impl Into<String>,
        behavior_id: impl Into<String>,
        fixture_id: impl Into<String>,
        dataset: BoundInput,
        scorer_id: impl Into<String>,
        scorer_digest_sha256: impl Into<String>,
        perturbation_controls: BTreeSet<PerturbationControl>,
        representative: bool,
    ) -> Self {
        let provenance =
            digest(format!("dataset-provenance|{}", dataset.digest_sha256()).as_bytes());
        Self::new_with_provenance(
            task_id,
            requirement_id,
            behavior_id,
            fixture_id,
            dataset,
            provenance,
            BTreeSet::new(),
            scorer_id,
            scorer_digest_sha256,
            perturbation_controls,
            representative,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_provenance(
        task_id: impl Into<String>,
        requirement_id: impl Into<String>,
        behavior_id: impl Into<String>,
        fixture_id: impl Into<String>,
        dataset: BoundInput,
        dataset_provenance_sha256: impl Into<String>,
        known_training_corpus_sha256s: BTreeSet<String>,
        scorer_id: impl Into<String>,
        scorer_digest_sha256: impl Into<String>,
        perturbation_controls: BTreeSet<PerturbationControl>,
        representative: bool,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            requirement_id: requirement_id.into(),
            behavior_id: behavior_id.into(),
            fixture_id: fixture_id.into(),
            dataset,
            dataset_provenance_sha256: dataset_provenance_sha256.into(),
            known_training_corpus_sha256s,
            scorer_id: scorer_id.into(),
            scorer_digest_sha256: scorer_digest_sha256.into(),
            perturbation_controls,
            representative,
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn fixture_id(&self) -> &str {
        &self.fixture_id
    }

    fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        for (field, value) in [
            ("task", self.task_id.as_str()),
            ("requirement", self.requirement_id.as_str()),
            ("behavior", self.behavior_id.as_str()),
            ("fixture", self.fixture_id.as_str()),
            ("scorer", self.scorer_id.as_str()),
        ] {
            if !valid_identifier(value) {
                findings.push(format!("evaluation-{field}-id-invalid"));
            }
        }
        findings.extend(self.dataset.findings("evaluation-dataset"));
        if !valid_sha256(&self.dataset_provenance_sha256)
            || self
                .known_training_corpus_sha256s
                .iter()
                .any(|digest| !valid_sha256(digest))
        {
            findings.push("evaluation-dataset-provenance-invalid".to_owned());
        }
        if self
            .known_training_corpus_sha256s
            .contains(self.dataset.digest_sha256())
        {
            findings.push("evaluation-dataset-contamination-detected".to_owned());
        }
        if !valid_sha256(&self.scorer_digest_sha256) {
            findings.push("evaluation-scorer-digest-invalid".to_owned());
        }
        for required in PerturbationControl::REQUIRED {
            if !self.perturbation_controls.contains(&required) {
                findings.push(format!(
                    "evaluation-perturbation-control-missing:{}",
                    required.label()
                ));
            }
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        let controls = self
            .perturbation_controls
            .iter()
            .copied()
            .map(PerturbationControl::label)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.task_id,
            self.requirement_id,
            self.behavior_id,
            self.fixture_id,
            self.dataset.digest_fragment(),
            self.dataset_provenance_sha256,
            self.known_training_corpus_sha256s
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
            self.scorer_id,
            self.scorer_digest_sha256,
            controls,
            self.representative
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationSpec {
    live_context_id: String,
    candidate_id: String,
    spec_id: String,
    tasks: Vec<EvaluationTask>,
    task_set_sha256: String,
    spec_sha256: String,
    revision: u64,
}

impl EvaluationSpec {
    pub fn new(
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        spec_id: impl Into<String>,
        mut tasks: Vec<EvaluationTask>,
    ) -> Result<Self, EvaluationError> {
        if tasks.is_empty() || tasks.len() > MAX_TASKS {
            return Err(EvaluationError::new("evaluation-task-count-out-of-bounds"));
        }
        tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
        let live_context_id = live_context_id.into();
        let candidate_id = candidate_id.into();
        let spec_id = spec_id.into();
        let task_set_sha256 = digest(
            tasks
                .iter()
                .map(EvaluationTask::digest_fragment)
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        );
        let spec_sha256 = digest(
            format!("{live_context_id}|{candidate_id}|{spec_id}|{task_set_sha256}").as_bytes(),
        );
        Ok(Self {
            live_context_id,
            candidate_id,
            spec_id,
            tasks,
            task_set_sha256,
            spec_sha256,
            revision: 0,
        })
    }

    pub fn audit(&self, current_context_id: &str, current_candidate_id: &str) -> TaskAudit {
        let mut findings = Vec::new();
        if self.live_context_id != current_context_id {
            findings.push("evaluation-context-stale".to_owned());
        }
        if self.candidate_id != current_candidate_id {
            findings.push("evaluation-candidate-stale".to_owned());
        }
        if !valid_sha256(&self.live_context_id) || !valid_sha256(&self.candidate_id) {
            findings.push("evaluation-binding-invalid".to_owned());
        }
        if !valid_identifier(&self.spec_id) {
            findings.push("evaluation-spec-id-invalid".to_owned());
        }
        let mut task_ids = BTreeSet::new();
        let mut fixture_ids = BTreeSet::new();
        let mut dataset_digests = BTreeMap::<&str, &str>::new();
        let mut representative = 0usize;
        for task in &self.tasks {
            if !task_ids.insert(task.task_id.as_str()) {
                findings.push("evaluation-duplicate-task".to_owned());
            }
            if !fixture_ids.insert(task.fixture_id.as_str()) {
                findings.push("evaluation-duplicate-fixture".to_owned());
            }
            if task.representative {
                representative += 1;
            }
            if let Some(existing) =
                dataset_digests.insert(task.dataset.relative_path(), task.dataset.digest_sha256())
                && existing != task.dataset.digest_sha256()
            {
                findings.push("evaluation-dataset-path-conflict".to_owned());
            }
            findings.extend(task.findings());
        }
        if representative == 0 {
            findings.push("evaluation-representative-task-missing".to_owned());
        }
        if self.task_set_sha256
            != digest(
                self.tasks
                    .iter()
                    .map(EvaluationTask::digest_fragment)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .as_bytes(),
            )
            || self.spec_sha256
                != digest(
                    format!(
                        "{}|{}|{}|{}",
                        self.live_context_id, self.candidate_id, self.spec_id, self.task_set_sha256
                    )
                    .as_bytes(),
                )
        {
            findings.push("evaluation-spec-mutated".to_owned());
        }
        findings.sort();
        findings.dedup();
        TaskAudit {
            live_context_id: current_context_id.to_owned(),
            candidate_id: current_candidate_id.to_owned(),
            spec_sha256: self.spec_sha256.clone(),
            spec_revision: self.revision,
            eligible: findings.is_empty(),
            findings,
        }
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn live_context_id(&self) -> &str {
        &self.live_context_id
    }

    pub fn spec_id(&self) -> &str {
        &self.spec_id
    }

    pub fn spec_sha256(&self) -> &str {
        &self.spec_sha256
    }

    pub fn task_set_sha256(&self) -> &str {
        &self.task_set_sha256
    }

    #[cfg(test)]
    pub(crate) fn corrupt_candidate_for_test(&mut self, candidate_id: impl Into<String>) {
        self.candidate_id = candidate_id.into();
        self.revision = self.revision.saturating_add(1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TaskAudit {
    live_context_id: String,
    candidate_id: String,
    spec_sha256: String,
    spec_revision: u64,
    eligible: bool,
    findings: Vec<String>,
}

impl TaskAudit {
    pub fn eligible(&self) -> bool {
        self.eligible
    }

    pub fn findings(&self) -> &[String] {
        &self.findings
    }

    fn revalidate(&self, spec: &EvaluationSpec, context: &str, candidate: &str) -> bool {
        self.eligible
            && self.live_context_id == context
            && self.candidate_id == candidate
            && self.spec_sha256 == spec.spec_sha256
            && self.spec_revision == spec.revision
            && spec.audit(context, candidate).eligible
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorOutcome {
    Passed,
    Failed,
    Quarantined,
}

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

impl CapturedTaskObservation {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn captured(
        task_id: impl Into<String>,
        fixture_id: impl Into<String>,
        outcome: BehaviorOutcome,
        causal_code: impl Into<String>,
        score_earned: u64,
        score_possible: u64,
        work_units: u64,
        artifact: BoundInput,
        replay_artifact_digest_sha256: impl Into<String>,
        producer_id: impl Into<String>,
        observer_id: impl Into<String>,
        independent_grader_id: impl Into<String>,
        independent_score_earned: u64,
        independent_score_possible: u64,
        passed_perturbations: BTreeSet<PerturbationControl>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            fixture_id: fixture_id.into(),
            outcome,
            causal_code: causal_code.into(),
            score_earned,
            score_possible,
            work_units,
            artifact,
            replay_artifact_digest_sha256: replay_artifact_digest_sha256.into(),
            producer_id: producer_id.into(),
            observer_id: observer_id.into(),
            independent_grader_id: independent_grader_id.into(),
            independent_score_earned,
            independent_score_possible,
            passed_perturbations,
        }
    }
}

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

impl EvaluationRun {
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

impl std::fmt::Debug for PromotionReview {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PromotionReview")
            .field("contents", &"<redacted>")
            .finish()
    }
}

/// Implemented only by root-owned adapters inside this crate. The authority
/// supplies the current live binding and consumes each attestation once.
pub(crate) trait PromotionReviewAuthority {
    fn authority_id(&self) -> &str;
    fn reviewer_id(&self) -> &str;
    fn review_session_id(&self) -> &str;
    fn current_binding(&self) -> (&str, &str);
    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, EvaluationError>;
    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool;
}

impl PromotionReview {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn issue<A: PromotionReviewAuthority>(
        baseline: &EvaluationRun,
        candidate: &EvaluationRun,
        representative_journey: BoundInput,
        rollback_evidence: BoundInput,
        mut reviewed_artifacts: Vec<BoundInput>,
        authority: &mut A,
    ) -> Result<Self, EvaluationError> {
        let authority_id = authority.authority_id().to_owned();
        let reviewer_id = authority.reviewer_id().to_owned();
        let issuance_session_id = authority.review_session_id().to_owned();
        let (authority_context, authority_candidate) = authority.current_binding();
        let authority_context = authority_context.to_owned();
        let authority_candidate = authority_candidate.to_owned();
        reviewed_artifacts.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

        if !runs_comparable(baseline, candidate)
            || !run_revalidates(baseline)
            || !run_revalidates(candidate)
            || !valid_identifier(&reviewer_id)
            || !valid_identifier(&authority_id)
            || reviewer_id == authority_id
            || !valid_sha256(&issuance_session_id)
            || authority_context != candidate.live_context_id
            || authority_candidate != candidate.candidate_id
            || issuance_session_id == baseline.execution_session_id
            || issuance_session_id == candidate.execution_session_id
            || baseline.execution_session_id == candidate.execution_session_id
            || review_principal_conflicts(baseline, candidate, &reviewer_id)
            || review_principal_conflicts(baseline, candidate, &authority_id)
            || !review_evidence_is_complete(
                &representative_journey,
                &rollback_evidence,
                &reviewed_artifacts,
            )
        {
            return Err(EvaluationError::new("evaluation-review-issuance-refused"));
        }

        let binding_sha256 = promotion_review_binding(
            baseline,
            candidate,
            &reviewer_id,
            &authority_id,
            &issuance_session_id,
            &representative_journey,
            &rollback_evidence,
            &reviewed_artifacts,
        );
        let attestation_sha256 = authority.issue_attestation(&binding_sha256)?;
        let review_id =
            digest(format!("promotion-review|{binding_sha256}|{attestation_sha256}").as_bytes());
        if !valid_sha256(&attestation_sha256)
            || authority.authority_id() != authority_id
            || authority.reviewer_id() != reviewer_id
            || authority.review_session_id() != issuance_session_id
            || authority.current_binding()
                != (authority_context.as_str(), authority_candidate.as_str())
        {
            return Err(EvaluationError::new("evaluation-review-issuance-refused"));
        }

        Ok(Self {
            reviewer_id,
            authority_id,
            issuance_session_id,
            live_context_id: candidate.live_context_id.clone(),
            baseline_candidate_id: baseline.candidate_id.clone(),
            candidate_id: candidate.candidate_id.clone(),
            baseline_run_sha256: baseline.run_sha256.clone(),
            candidate_run_sha256: candidate.run_sha256.clone(),
            baseline_spec_id: baseline.spec_id.clone(),
            candidate_spec_id: candidate.spec_id.clone(),
            baseline_spec_sha256: baseline.spec_sha256.clone(),
            candidate_spec_sha256: candidate.spec_sha256.clone(),
            task_set_sha256: candidate.task_set_sha256.clone(),
            baseline_execution_session_id: baseline.execution_session_id.clone(),
            candidate_execution_session_id: candidate.execution_session_id.clone(),
            representative_journey,
            rollback_evidence,
            reviewed_artifacts,
            binding_sha256,
            review_id,
            attestation_sha256,
        })
    }

    #[cfg(test)]
    pub(crate) fn substitute_candidate_for_test(&mut self, candidate_id: impl Into<String>) {
        self.candidate_id = candidate_id.into();
    }

    #[cfg(test)]
    pub(crate) fn substitute_journey_for_test(&mut self, journey: BoundInput) {
        self.representative_journey = journey;
    }

    #[cfg(test)]
    pub(crate) fn substitute_candidate_spec_for_test(&mut self, spec_sha256: impl Into<String>) {
        self.candidate_spec_sha256 = spec_sha256.into();
    }

    #[cfg(test)]
    pub(crate) fn inject_debug_sentinels_for_test(&mut self) -> Vec<String> {
        let mut sentinels = Vec::new();
        let mut marker = |label: &str| {
            let value = format!("opaque-promotion-review-{label}-marker");
            sentinels.push(value.clone());
            value
        };

        self.reviewer_id = marker("reviewer-id");
        self.authority_id = marker("authority-id");
        self.issuance_session_id = marker("issuance-session-id");
        self.live_context_id = marker("live-context-id");
        self.baseline_candidate_id = marker("baseline-candidate-id");
        self.candidate_id = marker("candidate-id");
        self.baseline_run_sha256 = marker("baseline-run-sha256");
        self.candidate_run_sha256 = marker("candidate-run-sha256");
        self.baseline_spec_id = marker("baseline-spec-id");
        self.candidate_spec_id = marker("candidate-spec-id");
        self.baseline_spec_sha256 = marker("baseline-spec-sha256");
        self.candidate_spec_sha256 = marker("candidate-spec-sha256");
        self.task_set_sha256 = marker("task-set-sha256");
        self.baseline_execution_session_id = marker("baseline-execution-session-id");
        self.candidate_execution_session_id = marker("candidate-execution-session-id");
        self.representative_journey = BoundInput {
            relative_path: marker("representative-journey-path"),
            digest_sha256: marker("representative-journey-digest"),
            byte_length: 78_101,
            link_count: 78_111,
            kind: InputKind::Directory,
        };
        self.rollback_evidence = BoundInput {
            relative_path: marker("rollback-evidence-path"),
            digest_sha256: marker("rollback-evidence-digest"),
            byte_length: 78_102,
            link_count: 78_112,
            kind: InputKind::Symlink,
        };
        self.reviewed_artifacts = vec![BoundInput {
            relative_path: marker("reviewed-artifacts-path"),
            digest_sha256: marker("reviewed-artifacts-digest"),
            byte_length: 78_103,
            link_count: 78_113,
            kind: InputKind::Special,
        }];
        self.binding_sha256 = marker("binding-sha256");
        self.review_id = marker("review-id");
        self.attestation_sha256 = marker("attestation-sha256");
        drop(marker);

        sentinels.extend(
            [
                "78101",
                "78102",
                "78103",
                "78111",
                "78112",
                "78113",
                "Directory",
                "Symlink",
                "Special",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        sentinels
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStatus {
    ImprovementCandidate,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PromotionDecision {
    pub baseline_candidate_id: String,
    pub candidate_id: String,
    pub reviewer_id: String,
    pub status: PromotionStatus,
    pub baseline_run_sha256: String,
    pub candidate_run_sha256: String,
    pub reasons: Vec<String>,
    pub claim_ceiling: String,
}

impl PromotionDecision {
    pub(crate) fn reconcile<A: PromotionReviewAuthority>(
        baseline: &EvaluationRun,
        candidate: &EvaluationRun,
        review: &PromotionReview,
        authority: &mut A,
    ) -> Self {
        let mut reasons = Vec::new();
        let pair_is_current = runs_comparable(baseline, candidate)
            && run_revalidates(baseline)
            && run_revalidates(candidate);
        if !pair_is_current {
            reasons.push("evaluation-pair-not-comparable".to_owned());
        }

        let principal_is_independent = valid_identifier(&review.reviewer_id)
            && valid_identifier(&review.authority_id)
            && review.reviewer_id != review.authority_id
            && review.issuance_session_id != baseline.execution_session_id
            && review.issuance_session_id != candidate.execution_session_id
            && baseline.execution_session_id != candidate.execution_session_id
            && !review_principal_conflicts(baseline, candidate, &review.reviewer_id)
            && !review_principal_conflicts(baseline, candidate, &review.authority_id);
        if !principal_is_independent {
            reasons.push("evaluation-review-not-independent".to_owned());
        }

        let evidence_is_complete = review_evidence_is_complete(
            &review.representative_journey,
            &review.rollback_evidence,
            &review.reviewed_artifacts,
        );
        if !evidence_is_complete {
            reasons.push("evaluation-review-evidence-invalid".to_owned());
        }

        let (current_context, current_candidate) = authority.current_binding();
        let current_context = current_context.to_owned();
        let current_candidate = current_candidate.to_owned();
        let authority_is_current = authority.authority_id() == review.authority_id
            && authority.reviewer_id() == review.reviewer_id
            && authority.review_session_id() == review.issuance_session_id
            && current_context == review.live_context_id
            && current_candidate == review.candidate_id;
        if !authority_is_current {
            reasons.push("evaluation-review-authority-stale".to_owned());
        }

        let expected_binding = promotion_review_binding(
            baseline,
            candidate,
            &review.reviewer_id,
            &review.authority_id,
            &review.issuance_session_id,
            &review.representative_journey,
            &review.rollback_evidence,
            &review.reviewed_artifacts,
        );
        let expected_review_id = digest(
            format!(
                "promotion-review|{}|{}",
                expected_binding, review.attestation_sha256
            )
            .as_bytes(),
        );
        let binding_is_current = review_matches_runs(review, baseline, candidate)
            && valid_sha256(&review.attestation_sha256)
            && review.binding_sha256 == expected_binding
            && review.review_id == expected_review_id;
        if !binding_is_current {
            reasons.push("evaluation-review-binding-stale-or-substituted".to_owned());
        }

        if pair_is_current
            && principal_is_independent
            && evidence_is_complete
            && authority_is_current
            && binding_is_current
        {
            let attestation_accepted = authority.verify_and_consume(
                &review.binding_sha256,
                &review.reviewer_id,
                &review.review_id,
                &review.attestation_sha256,
            );
            let authority_remained_current = authority.authority_id() == review.authority_id
                && authority.reviewer_id() == review.reviewer_id
                && authority.review_session_id() == review.issuance_session_id
                && authority.current_binding()
                    == (current_context.as_str(), current_candidate.as_str());
            if !attestation_accepted || !authority_remained_current {
                reasons.push("evaluation-review-attestation-invalid-or-replayed".to_owned());
            }
        }

        let baseline_by_id = baseline
            .results
            .iter()
            .map(|result| (result.task_id.as_str(), result))
            .collect::<BTreeMap<_, _>>();
        let candidate_by_id = candidate
            .results
            .iter()
            .map(|result| (result.task_id.as_str(), result))
            .collect::<BTreeMap<_, _>>();
        if baseline_by_id.keys().collect::<Vec<_>>() != candidate_by_id.keys().collect::<Vec<_>>() {
            reasons.push("evaluation-task-set-drift".to_owned());
        }
        let baseline_failures = baseline
            .results
            .iter()
            .filter(|result| result.outcome != BehaviorOutcome::Passed)
            .count();
        let candidate_failures = candidate
            .results
            .iter()
            .filter(|result| result.outcome != BehaviorOutcome::Passed)
            .count();
        if candidate_failures >= baseline_failures {
            reasons.push("evaluation-no-representative-behavior-improvement".to_owned());
        }
        if candidate
            .results
            .iter()
            .any(|result| result.representative && result.outcome != BehaviorOutcome::Passed)
        {
            reasons.push("evaluation-representative-regression".to_owned());
        }
        for (task_id, candidate_result) in &candidate_by_id {
            let Some(baseline_result) = baseline_by_id.get(task_id) else {
                continue;
            };
            if u128::from(candidate_result.score_earned)
                * u128::from(baseline_result.score_possible)
                < u128::from(baseline_result.score_earned)
                    * u128::from(candidate_result.score_possible)
                || (baseline_result.outcome == BehaviorOutcome::Passed
                    && candidate_result.outcome != BehaviorOutcome::Passed)
            {
                reasons.push(format!("evaluation-task-regressed:{task_id}"));
            }
            for control in PerturbationControl::REQUIRED {
                if !candidate_result.passed_perturbations.contains(&control) {
                    reasons.push(format!(
                        "evaluation-reward-hacking-control-missing:{}",
                        control.label()
                    ));
                }
            }
        }
        let (baseline_earned, baseline_possible) = totals(&baseline.results);
        let (candidate_earned, candidate_possible) = totals(&candidate.results);
        if u128::from(candidate_earned) * u128::from(baseline_possible)
            <= u128::from(baseline_earned) * u128::from(candidate_possible)
        {
            reasons.push("evaluation-score-not-improved".to_owned());
        }
        reasons.sort();
        reasons.dedup();
        Self {
            baseline_candidate_id: baseline.candidate_id.clone(),
            candidate_id: candidate.candidate_id.clone(),
            reviewer_id: review.reviewer_id.clone(),
            status: if reasons.is_empty() {
                PromotionStatus::ImprovementCandidate
            } else {
                PromotionStatus::Rejected
            },
            baseline_run_sha256: baseline.run_sha256.clone(),
            candidate_run_sha256: candidate.run_sha256.clone(),
            reasons,
            claim_ceiling: "improvement_candidate_not_product_completion".to_owned(),
        }
    }
}

fn runs_comparable(baseline: &EvaluationRun, candidate: &EvaluationRun) -> bool {
    baseline.live_context_id == candidate.live_context_id
        && baseline.candidate_id != candidate.candidate_id
        && baseline.spec_id == candidate.spec_id
        && baseline.task_set_sha256 == candidate.task_set_sha256
        && baseline.execution_session_id != candidate.execution_session_id
        && baseline
            .results
            .iter()
            .map(|result| result.task_id.as_str())
            .eq(candidate
                .results
                .iter()
                .map(|result| result.task_id.as_str()))
}

fn run_revalidates(run: &EvaluationRun) -> bool {
    let mut task_ids = BTreeSet::new();
    let mut fixture_ids = BTreeSet::new();
    valid_sha256(&run.live_context_id)
        && valid_sha256(&run.candidate_id)
        && valid_identifier(&run.spec_id)
        && valid_sha256(&run.task_set_sha256)
        && valid_sha256(&run.execution_session_id)
        && run.spec_sha256
            == digest(
                format!(
                    "{}|{}|{}|{}",
                    run.live_context_id, run.candidate_id, run.spec_id, run.task_set_sha256
                )
                .as_bytes(),
            )
        && !run.results.is_empty()
        && run.results.iter().all(|result| {
            task_ids.insert(result.task_id.as_str())
                && fixture_ids.insert(result.fixture_id.as_str())
                && valid_identifier(&result.task_id)
                && valid_identifier(&result.fixture_id)
                && valid_identifier(&result.causal_code)
                && valid_sha256(&result.artifact_digest_sha256)
                && valid_identifier(&result.producer_id)
                && valid_identifier(&result.observer_id)
                && valid_identifier(&result.scorer_id)
                && valid_identifier(&result.independent_grader_id)
                && result.producer_id != result.observer_id
                && result.observer_id != result.scorer_id
                && result.independent_grader_id != result.scorer_id
                && result.independent_grader_id != result.producer_id
                && result.independent_grader_id != result.observer_id
                && result.score_possible > 0
                && result.score_earned <= result.score_possible
                && result.independent_score_possible > 0
                && result.independent_score_earned <= result.independent_score_possible
                && u128::from(result.score_earned) * u128::from(result.independent_score_possible)
                    == u128::from(result.independent_score_earned)
                        * u128::from(result.score_possible)
                && result.work_units > 0
                && PerturbationControl::REQUIRED
                    .iter()
                    .all(|control| result.passed_perturbations.contains(control))
        })
        && run.run_sha256
            == run_digest_fields(
                &run.spec_sha256,
                &run.live_context_id,
                &run.candidate_id,
                &run.execution_session_id,
                &run.results,
            )
}

fn review_principal_conflicts(
    baseline: &EvaluationRun,
    candidate: &EvaluationRun,
    principal: &str,
) -> bool {
    baseline
        .results
        .iter()
        .chain(&candidate.results)
        .any(|result| {
            result.producer_id == principal
                || result.observer_id == principal
                || result.scorer_id == principal
                || result.independent_grader_id == principal
        })
}

fn review_evidence_is_complete(
    representative_journey: &BoundInput,
    rollback_evidence: &BoundInput,
    reviewed_artifacts: &[BoundInput],
) -> bool {
    if !representative_journey
        .findings("evaluation-review-journey")
        .is_empty()
        || !rollback_evidence
            .findings("evaluation-review-rollback")
            .is_empty()
        || reviewed_artifacts.is_empty()
        || reviewed_artifacts.len() > 64
        || reviewed_artifacts
            .iter()
            .any(|input| !input.findings("evaluation-reviewed-artifact").is_empty())
        || reviewed_artifacts
            .windows(2)
            .any(|pair| pair[0].relative_path >= pair[1].relative_path)
    {
        return false;
    }

    let mut paths = BTreeSet::new();
    let mut digests = BTreeSet::new();
    std::iter::once(representative_journey)
        .chain(std::iter::once(rollback_evidence))
        .chain(reviewed_artifacts)
        .all(|input| {
            paths.insert(input.relative_path.as_str())
                && digests.insert(input.digest_sha256.as_str())
        })
}

#[allow(clippy::too_many_arguments)]
fn promotion_review_binding(
    baseline: &EvaluationRun,
    candidate: &EvaluationRun,
    reviewer_id: &str,
    authority_id: &str,
    issuance_session_id: &str,
    representative_journey: &BoundInput,
    rollback_evidence: &BoundInput,
    reviewed_artifacts: &[BoundInput],
) -> String {
    let artifacts = reviewed_artifacts
        .iter()
        .map(BoundInput::digest_fragment)
        .collect::<Vec<_>>()
        .join("\n");
    digest(
        format!(
            "promotion-review-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            reviewer_id,
            authority_id,
            issuance_session_id,
            candidate.live_context_id,
            baseline.candidate_id,
            candidate.candidate_id,
            baseline.run_sha256,
            candidate.run_sha256,
            baseline.spec_id,
            candidate.spec_id,
            baseline.spec_sha256,
            candidate.spec_sha256,
            baseline.task_set_sha256,
            candidate.task_set_sha256,
            baseline.execution_session_id,
            candidate.execution_session_id,
            representative_journey.digest_fragment(),
            format!("{}|{}", rollback_evidence.digest_fragment(), artifacts),
        )
        .as_bytes(),
    )
}

fn review_matches_runs(
    review: &PromotionReview,
    baseline: &EvaluationRun,
    candidate: &EvaluationRun,
) -> bool {
    review.live_context_id == candidate.live_context_id
        && review.baseline_candidate_id == baseline.candidate_id
        && review.candidate_id == candidate.candidate_id
        && review.baseline_run_sha256 == baseline.run_sha256
        && review.candidate_run_sha256 == candidate.run_sha256
        && review.baseline_spec_id == baseline.spec_id
        && review.candidate_spec_id == candidate.spec_id
        && review.baseline_spec_sha256 == baseline.spec_sha256
        && review.candidate_spec_sha256 == candidate.spec_sha256
        && review.task_set_sha256 == candidate.task_set_sha256
        && review.task_set_sha256 == baseline.task_set_sha256
        && review.baseline_execution_session_id == baseline.execution_session_id
        && review.candidate_execution_session_id == candidate.execution_session_id
}

fn totals(results: &[EvaluationTaskResult]) -> (u64, u64) {
    results.iter().fold((0, 0), |(earned, possible), result| {
        (
            earned.saturating_add(result.score_earned),
            possible.saturating_add(result.score_possible),
        )
    })
}

fn run_digest(
    spec: &EvaluationSpec,
    execution_session_id: &str,
    results: &[EvaluationTaskResult],
) -> String {
    run_digest_fields(
        &spec.spec_sha256,
        &spec.live_context_id,
        &spec.candidate_id,
        execution_session_id,
        results,
    )
}

fn run_digest_fields(
    spec_sha256: &str,
    live_context_id: &str,
    candidate_id: &str,
    execution_session_id: &str,
    results: &[EvaluationTaskResult],
) -> String {
    let rows = results
        .iter()
        .map(|result| {
            let controls = result
                .passed_perturbations
                .iter()
                .copied()
                .map(PerturbationControl::label)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{}|{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
                result.task_id,
                result.fixture_id,
                result.representative,
                result.outcome,
                result.causal_code,
                result.score_earned,
                result.score_possible,
                result.work_units,
                result.artifact_digest_sha256,
                result.producer_id,
                result.observer_id,
                result.scorer_id,
                result.independent_grader_id,
                result.independent_score_earned,
                result.independent_score_possible,
                controls,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    digest(
        format!("{spec_sha256}|{live_context_id}|{candidate_id}|{execution_session_id}|{rows}")
            .as_bytes(),
    )
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PATH_BYTES
        && !value.starts_with('/')
        && !value.contains('\\')
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}
