pub(crate) use super::production_input::ProductionExecutionRequest;
use super::production_input::ProductionSpecPermit;
use super::{
    CanonicalEvaluationFailure, CanonicalEvaluationRun, CapturedTaskObservation, EvaluationError,
    EvaluationEventKind, EvaluationExecutor, EvaluationRun, EvaluationTask,
    PrivacySafeEvaluationEvent, PrivacySafeEvaluationEventRecord, RuntimeConfiguration,
};
use super::{ExecutionReservationOutcome, FileEvaluationExecutionLedger, digest};
use crate::fixture_scheduler::{FixtureExecutionBinding, FixtureExecutionRecord};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fmt;

const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionRuntimeError {
    code: &'static str,
}

impl ProductionRuntimeError {
    pub(crate) fn new(code: &'static str) -> Self {
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
    pub dataset_digest_sha256: String,
    pub scorer_policy_digest_sha256: String,
    pub executable_digest_sha256: String,
}

pub(crate) mod bridge_authority {
    pub(crate) trait Sealed {}
}

pub(crate) trait FixtureEvaluationBridge: bridge_authority::Sealed {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<FixtureExecutionRecord, ProductionRuntimeError>;
}

struct FixtureSchedulerEvaluationExecutor<'a, B> {
    bridge: &'a mut B,
    permit: &'a ProductionSpecPermit<'a>,
    live_context_id: String,
    candidate_id: String,
    spec_sha256: String,
    execution_session_id: String,
    records: Vec<FixtureExecutionRecord>,
    record_identities: BTreeSet<String>,
    lease_ids: BTreeSet<String>,
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
        let material = self.permit.take_task_material(task)?;
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
            dataset_digest_sha256: material.dataset_digest_sha256().to_owned(),
            scorer_policy_digest_sha256: material.scorer_policy_digest_sha256().to_owned(),
            executable_digest_sha256: material.executable_digest_sha256().to_owned(),
        };
        let record = self
            .bridge
            .execute_fixture(&request)
            .map_err(|error| EvaluationError::new(error.code()))?;
        self.permit.revalidate()?;
        if record.binding != request.binding
            || record.fixture_id != request.fixture_id
            || record.artifact_byte_length == 0
            || record.artifact_byte_length > MAX_ARTIFACT_BYTES
            || !self
                .record_identities
                .insert(record.record_sha256().to_owned())
            || !self.lease_ids.insert(record.lease_id.clone())
        {
            return Err(EvaluationError::new(
                "evaluation-fixture-record-binding-invalid",
            ));
        }
        let observation = material.score(&record)?;
        self.records.push(record);
        Ok(observation)
    }
}

pub(crate) fn execute_production<B: FixtureEvaluationBridge>(
    permit: &ProductionSpecPermit<'_>,
    execution_session_id: impl Into<String>,
    runtime_configuration: RuntimeConfiguration,
    ledger: &mut FileEvaluationExecutionLedger,
    bridge: &mut B,
) -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
    permit.revalidate()?;
    let spec = permit.spec();
    let execution_session_id = execution_session_id.into();
    let binding = ledger.binding();
    if binding.live_context_id != spec.live_context_id()
        || binding.candidate_id != spec.candidate_id()
        || binding.spec_sha256 != spec.spec_sha256()
        || binding.task_set_sha256 != spec.task_set_sha256()
        || binding.execution_session_id != execution_session_id
        || binding.execution_material_set_sha256 != permit.material_set_sha256()
    {
        return Err(ProductionRuntimeError::new(
            "evaluation-production-ledger-binding-invalid",
        ));
    }
    let reservation_id = digest(
        format!(
            "production-execution|{}|{}",
            spec.spec_sha256(),
            execution_session_id
        )
        .as_bytes(),
    );
    if ledger
        .reserve_outcome(&reservation_id)
        .map_err(|error| ProductionRuntimeError::new(error.code()))?
        != ExecutionReservationOutcome::Acquired
    {
        return Err(ProductionRuntimeError::new(
            "evaluation-production-execution-replayed",
        ));
    }
    let mut executor = FixtureSchedulerEvaluationExecutor {
        bridge,
        permit,
        live_context_id: spec.live_context_id().to_owned(),
        candidate_id: spec.candidate_id().to_owned(),
        spec_sha256: spec.spec_sha256().to_owned(),
        execution_session_id,
        records: Vec::new(),
        record_identities: BTreeSet::new(),
        lease_ids: BTreeSet::new(),
    };
    let result = (|| -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
        let run = EvaluationRun::execute_local(spec, permit.audit(), &mut executor)?;
        permit.revalidate()?;
        let records = std::mem::take(&mut executor.records);
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
        let canonical_run =
            CanonicalEvaluationRun::from_parts(&run, runtime_configuration, &records);
        let artifact_set_sha256 = digest(
            records
                .iter()
                .map(FixtureExecutionRecord::record_sha256)
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        );
        ledger
            .publish_result(run.run_sha256(), artifact_set_sha256)
            .map_err(|error| ProductionRuntimeError::new(error.code()))?;
        Ok(ProductionEvaluationRun {
            canonical_run,
            canonical_failures,
            events: vec![event],
            fixture_records: records,
        })
    })();
    match result {
        Ok(run) => Ok(run),
        Err(error) => {
            ledger
                .mark_interrupted(error.code())
                .map_err(|ledger_error| ProductionRuntimeError::new(ledger_error.code()))?;
            Err(error)
        }
    }
}
