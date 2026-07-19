use super::runtime::{
    FixtureEvaluationBridge, FixtureTaskRequest, ProductionEvaluationRun, ProductionRuntimeError,
};
use super::{
    CanonicalEvaluationFailure, CanonicalEvaluationRun, CapturedTaskObservation, EvaluationError,
    EvaluationEventKind, EvaluationExecutor, EvaluationRun, EvaluationTask,
    PrivacySafeEvaluationEvent, PrivacySafeEvaluationEventRecord, RuntimeConfiguration,
};
use crate::fixture_scheduler::{FixtureExecutionBinding, FixtureExecutionRecord};
use std::collections::BTreeSet;

const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024;

struct FixtureSchedulerEvaluationExecutor<'a, B> {
    bridge: &'a mut B,
    permit: &'a super::production_input::ProductionSpecPermit<'a>,
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

impl ExecutionOwner {
    /// The sole custody transition: reserve, execute through the sealed
    /// fixture bridge, publish authenticated identities, and settle terminally.
    fn execute_production<B: FixtureEvaluationBridge>(
        &mut self,
        permit: &super::production_input::ProductionSpecPermit<'_>,
        execution_session_id: impl Into<String>,
        runtime_configuration: RuntimeConfiguration,
        expected_binding: &super::EvaluationExecutionBinding,
        output_path: Option<&std::path::Path>,
        bridge: &mut B,
    ) -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
        permit.revalidate()?;
        let spec = permit.spec();
        let execution_session_id = execution_session_id.into();
        if self.0.binding != *expected_binding
            || expected_binding.live_context_id != spec.live_context_id()
            || expected_binding.candidate_id != spec.candidate_id()
            || expected_binding.spec_sha256 != spec.spec_sha256()
            || expected_binding.task_set_sha256 != spec.task_set_sha256()
            || expected_binding.execution_session_id != execution_session_id
            || expected_binding.execution_material_set_sha256 != permit.material_set_sha256()
        {
            return Err(ProductionRuntimeError::new(
                "evaluation-production-ledger-binding-invalid",
            ));
        }
        let reservation_id = super::digest(
            serde_json::to_string(expected_binding)
                .map_err(|_| ProductionRuntimeError::new("evaluation-production-binding-invalid"))?
                .as_bytes(),
        );
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
        let reservation = self
            .0
            .reserve_outcome(&reservation_id)
            .map_err(|error| ProductionRuntimeError::new(error.code()))?;
        match reservation {
            super::ExecutionReservationOutcome::Acquired => {}
            super::ExecutionReservationOutcome::Terminal {
                terminal_result, ..
            } => {
                publish_public_result(output_path, &terminal_result)?;
                return ProductionEvaluationRun::from_terminal(terminal_result);
            }
            _ => {
                return Err(ProductionRuntimeError::new(
                    "evaluation-production-execution-replayed",
                ));
            }
        }
        let effect = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let run = EvaluationRun::execute_local(spec, permit.audit(), &mut executor)
                .map_err(|error| error.code())?;
            permit.revalidate().map_err(|error| error.code())?;
            let records = std::mem::take(&mut executor.records);
            if records.len() != run.results().len() {
                return Err("evaluation-fixture-record-count-mismatch");
            }
            let canonical_failures: Vec<_> = run
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
            })
            .map_err(|error| error.code())?;
            let canonical_run =
                CanonicalEvaluationRun::from_parts(&run, runtime_configuration, &records);
            let artifact_set_sha256 = super::digest(
                records
                    .iter()
                    .map(FixtureExecutionRecord::record_sha256)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .as_bytes(),
            );
            let public_result = serde_json::to_vec(&serde_json::json!({
                "schema_version": "EvaluationRunResult-v1",
                "canonical_run": &canonical_run,
                "canonical_failures": &canonical_failures,
                "events": [&event],
                "claim_effect": "none",
            }))
            .map_err(|_| "evaluation-terminal-result-serialization-failed")?;
            Ok((
                run,
                canonical_run,
                canonical_failures,
                event,
                public_result,
                records,
                artifact_set_sha256,
            ))
        }))
        .unwrap_or(Err("evaluation-execution-panicked"));
        match effect {
            Ok((
                run,
                canonical_run,
                canonical_failures,
                event,
                public_result,
                records,
                artifacts,
            )) => {
                self.settle(run.run_sha256(), artifacts, &public_result, output_path)?;
                Ok(ProductionEvaluationRun::new(
                    canonical_run,
                    canonical_failures,
                    vec![event],
                    public_result,
                    records,
                ))
            }
            Err(code) => Err(self.interrupt(code)),
        }
    }

    fn settle(
        &mut self,
        run_sha256: &str,
        artifacts: String,
        terminal_result: &[u8],
        output_path: Option<&std::path::Path>,
    ) -> Result<(), ProductionRuntimeError> {
        publish_public_result(output_path, terminal_result)?;
        if let Err(error) =
            self.publish_terminal_result(run_sha256, artifacts, terminal_result.to_vec())
        {
            return Err(self.interrupt(error.code()));
        }
        if let Err(error) = self.0.complete() {
            return Err(self.interrupt(error.code()));
        }
        Ok(())
    }

    fn interrupt(&mut self, causal_code: &'static str) -> ProductionRuntimeError {
        match self.0.mark_interrupted(causal_code) {
            Ok(()) => ProductionRuntimeError::new(causal_code),
            Err(error) => ProductionRuntimeError::new(error.code()),
        }
    }
}
