use super::*;

impl crate::evaluation::runtime::bridge_authority::Sealed for ScheduledFixtureEvaluationBridge {}

impl FixtureEvaluationBridge for ScheduledFixtureEvaluationBridge {
    fn execute_fixture(
        &mut self,
        request: &FixtureTaskRequest,
    ) -> Result<crate::fixture_scheduler::FixtureExecutionRecord, ProductionRuntimeError> {
        let invocation = self
            .invocations
            .get(&request.fixture_id)
            .ok_or_else(|| ProductionRuntimeError::bridge("evaluation-fixture-not-configured"))?;
        let fixture = FixtureSpec::new(
            &request.fixture_id,
            FixtureKind::Positive,
            "evaluation-production-execution",
            BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Port]),
            ExpectedOutcome::pass(0),
            false,
        )
        .map_err(|_| ProductionRuntimeError::bridge("evaluation-fixture-spec-invalid"))?;
        let adapter = FixtureCaptureAdapter::issue_evaluation(
            &fixture,
            FixtureCaptureRequest {
                executable: invocation.executable.clone(),
                arguments: invocation.arguments.clone(),
                output_limit: invocation.output_limit,
                required_output: invocation.required_output.clone(),
                binding: request.binding.clone(),
            },
            &invocation.artifact_name,
        )
        .map_err(|_| ProductionRuntimeError::bridge("evaluation-fixture-permit-refused"))?;
        let lease_id = self
            .scheduler
            .schedule([fixture])
            .map_err(|_| ProductionRuntimeError::bridge("evaluation-fixture-schedule-failed"))?
            .pop()
            .ok_or_else(|| ProductionRuntimeError::bridge("evaluation-fixture-lease-missing"))?;
        let (disposition, _record) = match self.scheduler.execute_recorded(&lease_id, &adapter) {
            Ok(executed) => executed,
            Err(_) => {
                if self.scheduler.recover(&lease_id).is_err() {
                    self.recovery_required.insert(lease_id);
                    return Err(ProductionRuntimeError::bridge(
                        "evaluation-fixture-recovery-required",
                    ));
                }
                return Err(ProductionRuntimeError::bridge(
                    "evaluation-fixture-execution-failed",
                ));
            }
        };
        if disposition != RunDisposition::Accepted {
            if disposition == RunDisposition::CleanupFailure {
                self.recovery_required.insert(lease_id.clone());
                return Err(ProductionRuntimeError::bridge(
                    "evaluation-fixture-recovery-required",
                ));
            }
            return Err(ProductionRuntimeError::bridge(
                "evaluation-fixture-not-terminally-accepted",
            ));
        }
        Ok(record)
    }
}

#[cfg(test)]
#[path = "evaluation_bridge_tests.rs"]
mod tests;
