use super::*;

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
        let (disposition, record) = match self.scheduler.execute_recorded(&lease_id, &adapter) {
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
                self.recovery_records.insert(lease_id, record);
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
mod tests {
    use super::super::{
        FixtureCaptureAdapter, ScheduledFixtureEvaluationBridge, ScheduledFixtureInvocation,
    };
    use crate::evaluation::runtime::execute_production;
    use crate::evaluation::{
        BoundInput, EvaluationSpec, EvaluationTask, EvaluationTaskDefinition, PerturbationControl,
        RuntimeConfiguration,
    };
    use crate::fixture_scheduler::{
        ExpectedOutcome, FixtureKind, FixtureScheduler, FixtureSpec, ResourceKind, RunDisposition,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hul-fixture-capture-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn spec(id: &str, expected: ExpectedOutcome) -> FixtureSpec {
        FixtureSpec::new(
            id,
            if matches!(
                expected.verdict,
                crate::fixture_scheduler::OutcomeVerdict::Pass
            ) {
                FixtureKind::Positive
            } else {
                FixtureKind::Negative
            },
            "capture-execution",
            BTreeSet::from([ResourceKind::File, ResourceKind::Env, ResourceKind::Port]),
            expected,
            false,
        )
        .unwrap()
    }

    fn sha(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    #[test]
    fn pinned_fixture_adapter_executes_and_derives_acceptance() {
        let root = root("accept");
        let fixture = spec("executed", ExpectedOutcome::pass(2));
        let adapter = FixtureCaptureAdapter::issue(
            &fixture,
            "/usr/bin/true".into(),
            Vec::<OsString>::new(),
            4096,
            Vec::new(),
        )
        .unwrap();
        let mut scheduler = FixtureScheduler::new(&root);
        let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
        let disposition = scheduler.execute(&lease, &adapter).unwrap();
        #[cfg(target_os = "freebsd")]
        assert_eq!(disposition, RunDisposition::Accepted);
        #[cfg(not(target_os = "freebsd"))]
        assert_eq!(disposition, RunDisposition::CleanupFailure);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn nonzero_pinned_fixture_derives_only_the_bound_causal_control() {
        let root = root("negative");
        let fixture = spec(
            "negative",
            ExpectedOutcome::causal_failure("intentional", 1),
        );
        let adapter = FixtureCaptureAdapter::issue(
            &fixture,
            "/usr/bin/false".into(),
            Vec::<OsString>::new(),
            4096,
            Vec::new(),
        )
        .unwrap();
        let mut scheduler = FixtureScheduler::new(&root);
        let lease = scheduler.schedule([fixture]).unwrap().pop().unwrap();
        let disposition = scheduler.execute(&lease, &adapter).unwrap();
        #[cfg(target_os = "freebsd")]
        assert_eq!(disposition, RunDisposition::CausalFailure);
        #[cfg(not(target_os = "freebsd"))]
        assert_eq!(disposition, RunDisposition::CleanupFailure);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn scheduled_evaluation_bridge_captures_before_cleanup_and_requires_recovery() {
        let root = root("evaluation");
        std::fs::create_dir(&root).unwrap();
        let executable = root.join("evaluation-fixture.sh");
        std::fs::write(
            &executable,
            r#"#!/bin/sh
printf '%s' '{"schema_version":"EvaluationFixtureArtifact-v1","task_id":"core","fixture_id":"fixture-core","outcome":"passed","causal_code":"behavioral-pass","score_earned":10,"score_possible":10,"work_units":3,"producer_id":"fixture-producer","observer_id":"fixture-observer","independent_grader_id":"independent-grader","independent_score_earned":10,"independent_score_possible":10,"passed_perturbations":["verbosity","proof_artifact","receipt_production","test_manipulation","score_only"]}' > file/result.json
"#,
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let task = EvaluationTask::new(EvaluationTaskDefinition {
            task_id: "core".to_owned(),
            requirement_id: "REQ-core".to_owned(),
            behavior_id: "behavior-core".to_owned(),
            fixture_id: "fixture-core".to_owned(),
            dataset: BoundInput::regular("datasets/core.json", sha('d'), 128),
            scorer_id: "scorer-core".to_owned(),
            scorer_digest_sha256: sha('e'),
            perturbation_controls: PerturbationControl::REQUIRED.into_iter().collect(),
            representative: true,
        });
        let spec = EvaluationSpec::new(sha('a'), sha('b'), "production-suite", vec![task]).unwrap();
        let audit = spec.audit(&sha('a'), &sha('b'));
        let invocation = ScheduledFixtureInvocation::new(
            executable,
            Vec::new(),
            16 * 1024,
            Vec::new(),
            "result.json",
        );
        let mut bridge = ScheduledFixtureEvaluationBridge::new(
            &root,
            BTreeMap::from([("fixture-core".to_owned(), invocation)]),
        );
        let error = execute_production(
            &spec,
            &audit,
            sha('c'),
            RuntimeConfiguration::all_unknown(),
            &mut bridge,
        )
        .unwrap_err();
        assert_eq!(error.code(), "evaluation-fixture-recovery-required");
        assert_eq!(bridge.recovery_required().len(), 1);
        assert_eq!(bridge.recovery_records().len(), 1);
        let record = bridge.recovery_records().values().next().unwrap();
        assert_eq!(record.artifact_relative_path, "file/result.json");
        assert!(record.artifact_byte_length > 0);
        std::fs::remove_dir_all(root).unwrap();
    }
}
