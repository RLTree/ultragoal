use super::records::sensitive_text;
use super::{
    BehaviorOutcome, BoundInput, CanonicalEvaluationFailure, CanonicalEvaluationRun,
    CapturedTaskObservation, CapturedTaskObservationRecord, EvaluationError, EvaluationEventKind,
    EvaluationExecutor, EvaluationRun, EvaluationSpec, EvaluationTask, PerturbationControl,
    PrivacySafeEvaluationEvent, PrivacySafeEvaluationEventRecord, RuntimeConfiguration, TaskAudit,
};
use crate::fixture_scheduler::{FixtureExecutionBinding, FixtureExecutionRecord};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionRuntimeError {
    code: &'static str,
}

impl ProductionRuntimeError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub(crate) fn bridge(code: &'static str) -> Self {
        Self::new(code)
    }
}

impl fmt::Display for ProductionRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for ProductionRuntimeError {}

impl From<EvaluationError> for ProductionRuntimeError {
    fn from(value: EvaluationError) -> Self {
        Self::new(value.code())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProductionEvaluationRun {
    pub canonical_run: CanonicalEvaluationRun,
    pub canonical_failures: Vec<CanonicalEvaluationFailure>,
    pub events: Vec<PrivacySafeEvaluationEvent>,
    fixture_records: Vec<FixtureExecutionRecord>,
}

impl ProductionEvaluationRun {
    pub fn fixture_records(&self) -> &[FixtureExecutionRecord] {
        &self.fixture_records
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FixtureTaskRequest {
    pub binding: FixtureExecutionBinding,
    pub fixture_id: String,
}

/// Crate-controlled bridge. Public callers cannot substitute a callback that
/// mints observations; the production implementation is the confined fixture
/// scheduler/capture adapter.
pub(crate) trait FixtureEvaluationBridge {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<FixtureExecutionRecord, ProductionRuntimeError>;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureArtifactEnvelope {
    schema_version: String,
    task_id: String,
    fixture_id: String,
    outcome: BehaviorOutcome,
    causal_code: String,
    score_earned: u64,
    score_possible: u64,
    work_units: u64,
    producer_id: String,
    observer_id: String,
    independent_grader_id: String,
    independent_score_earned: u64,
    independent_score_possible: u64,
    passed_perturbations: BTreeSet<PerturbationControl>,
}

struct FixtureSchedulerEvaluationExecutor<'a, B> {
    bridge: &'a mut B,
    live_context_id: String,
    candidate_id: String,
    spec_sha256: String,
    execution_session_id: String,
    records: Vec<FixtureExecutionRecord>,
}

impl<B: FixtureEvaluationBridge> EvaluationExecutor for FixtureSchedulerEvaluationExecutor<'_, B> {
    fn binding(&self) -> (&str, &str) {
        (&self.live_context_id, &self.candidate_id)
    }

    fn session_id(&self) -> &str {
        &self.execution_session_id
    }

    fn execute(
        &mut self,
        task: &EvaluationTask,
    ) -> Result<CapturedTaskObservation, EvaluationError> {
        let request = FixtureTaskRequest {
            binding: FixtureExecutionBinding::new(
                &self.live_context_id,
                &self.candidate_id,
                &self.spec_sha256,
                task.task_id(),
                &self.execution_session_id,
            )
            .map_err(|_| EvaluationError::new("evaluation-fixture-binding-invalid"))?,
            fixture_id: task.fixture_id().to_owned(),
        };
        let record = self
            .bridge
            .execute_fixture(&request)
            .map_err(|error| EvaluationError::new(error.code()))?;
        if record.binding != request.binding
            || record.fixture_id != request.fixture_id
            || record.artifact_byte_length == 0
            || record.artifact_byte_length > MAX_ARTIFACT_BYTES
            || record.artifact_byte_length != record.artifact_bytes().len() as u64
            || sensitive_text(record.artifact_bytes())
        {
            return Err(EvaluationError::new(
                "evaluation-fixture-record-binding-invalid",
            ));
        }
        let envelope: FixtureArtifactEnvelope = serde_json::from_slice(record.artifact_bytes())
            .map_err(|_| EvaluationError::new("evaluation-fixture-artifact-invalid"))?;
        if envelope.schema_version != "EvaluationFixtureArtifact-v1"
            || envelope.task_id != task.task_id()
            || envelope.fixture_id != task.fixture_id()
        {
            return Err(EvaluationError::new("evaluation-fixture-artifact-stale"));
        }
        let artifact = BoundInput::regular(
            format!("artifacts/{}", record.artifact_relative_path),
            record.artifact_digest_sha256.clone(),
            record.artifact_byte_length,
        );
        let observation = CapturedTaskObservation::captured(CapturedTaskObservationRecord {
            task_id: envelope.task_id,
            fixture_id: envelope.fixture_id,
            outcome: envelope.outcome,
            causal_code: envelope.causal_code,
            score_earned: envelope.score_earned,
            score_possible: envelope.score_possible,
            work_units: envelope.work_units,
            artifact,
            replay_artifact_digest_sha256: record.artifact_digest_sha256.clone(),
            producer_id: envelope.producer_id,
            observer_id: envelope.observer_id,
            independent_grader_id: envelope.independent_grader_id,
            independent_score_earned: envelope.independent_score_earned,
            independent_score_possible: envelope.independent_score_possible,
            passed_perturbations: envelope.passed_perturbations,
        });
        self.records.push(record);
        Ok(observation)
    }
}

pub(crate) fn execute_production<B: FixtureEvaluationBridge>(
    spec: &EvaluationSpec,
    audit: &TaskAudit,
    execution_session_id: impl Into<String>,
    runtime_configuration: RuntimeConfiguration,
    bridge: &mut B,
) -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
    let mut executor = FixtureSchedulerEvaluationExecutor {
        bridge,
        live_context_id: spec.live_context_id().to_owned(),
        candidate_id: spec.candidate_id().to_owned(),
        spec_sha256: spec.spec_sha256().to_owned(),
        execution_session_id: execution_session_id.into(),
        records: Vec::new(),
    };
    let run = EvaluationRun::execute_local(spec, audit, &mut executor)?;
    let records = executor.records;
    if records.len() != run.results().len() {
        return Err(ProductionRuntimeError::new(
            "evaluation-fixture-record-count-mismatch",
        ));
    }
    let canonical_failures = run
        .harvest_failures()
        .iter()
        .map(CanonicalEvaluationFailure::from)
        .collect();
    let event = PrivacySafeEvaluationEvent::new(PrivacySafeEvaluationEventRecord {
        event_kind: EvaluationEventKind::ExecutionPublished,
        live_context_id: spec.live_context_id().to_owned(),
        candidate_id: spec.candidate_id().to_owned(),
        spec_sha256: spec.spec_sha256().to_owned(),
        session_id: run.execution_session_id.clone(),
        run_sha256: Some(run.run_sha256().to_owned()),
        causal_code: None,
    })?;
    let canonical_run = CanonicalEvaluationRun::from_parts(&run, runtime_configuration, &records);
    Ok(ProductionEvaluationRun {
        canonical_run,
        canonical_failures,
        events: vec![event],
        fixture_records: records,
    })
}
