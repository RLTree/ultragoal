use super::*;
use crate::fixture_scheduler::FixtureExecutionRecordCapture;

#[cfg(test)]
impl FixtureExecutor for FixtureCaptureAdapter {
    fn execute(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ObservedOutcome, FixtureScheduleError> {
        self.capture(fixture, lease, environment)
            .map(|executed| executed.observed)
    }
}

impl RecordedFixtureExecutor for FixtureCaptureAdapter {
    fn execute_recorded(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ExecutedFixture, FixtureScheduleError> {
        self.capture(fixture, lease, environment)
    }
}

impl FixtureCaptureAdapter {
    pub(crate) fn capture(
        &self,
        fixture: &FixtureSpec,
        lease: &IsolationLease,
        environment: &BTreeMap<String, String>,
    ) -> Result<ExecutedFixture, FixtureScheduleError> {
        if fixture.id != self.fixture_id || fixture.metadata_digest != self.fixture_digest {
            return Err(FixtureScheduleError::Integrity(
                "fixture execution permit does not bind this immutable specification".to_owned(),
            ));
        }
        let executable_bytes = self.execution_bytes()?;
        #[cfg(unix)]
        match self.executable_kind {
            PinnedExecutableKind::ProtectedNative => self.validate_protected_native()?,
            PinnedExecutableKind::PosixShellSource => Self::validate_posix_shell_substrate()?,
            #[cfg(test)]
            PinnedExecutableKind::TestNativeSnapshot => {}
        }
        let (exit, stdout, overflow, early_termination) = run_confined(ConfinedExecution {
            fixture,
            source_executable: &self.executable,
            executable_kind: self.executable_kind,
            executable_bytes: &executable_bytes,
            arguments: &self.arguments,
            cwd: lease.root(),
            environment,
            output_limit: self.output_limit,
            interrupt: &self.interrupt,
        })?;
        let output_redacted = sensitive_text(&stdout);
        let stdout_digest = digest(if output_redacted {
            b"<redacted>"
        } else {
            &stdout
        });
        // Stderr remains subject to the shared bounded-output budget but is
        // not retained by this adapter, so its canonical digest is empty.
        let stderr_digest = digest(b"");
        let artifact = match self.artifact_name.as_deref() {
            Some(name) => read_artifact(lease.root(), name, self.output_limit)?,
            None => stdout.clone(),
        };
        if sensitive_text(&artifact) {
            return Err(FixtureScheduleError::Integrity(
                "fixture artifact contains secret-shaped data".to_owned(),
            ));
        }
        let expected = &fixture.expected;
        let valid_exit = matches!(expected.verdict, OutcomeVerdict::Pass) == (exit == Some(0));
        let observed = if overflow {
            ObservedOutcome::failure("fixture-output-limit-exceeded", 0)
        } else if let Some(causal_code) = early_termination {
            ObservedOutcome::failure(causal_code, 0)
        } else if !valid_exit || stdout != self.required_output {
            ObservedOutcome::failure("fixture-observation-mismatch", 0)
        } else {
            observed_from(expected)
        };
        let artifact_relative_path = self
            .artifact_name
            .as_ref()
            .map(|name| format!("file/{name}"))
            .unwrap_or_else(|| "captured-output.bin".to_owned());
        let record = FixtureExecutionRecord::captured(FixtureExecutionRecordCapture {
            binding: self.binding.clone(),
            fixture_id: fixture.id.clone(),
            fixture_digest_sha256: fixture.metadata_digest.clone(),
            lease_id: lease.id().to_owned(),
            executable_digest_sha256: self.executable_digest.clone(),
            executable_identity_sha256: self.executable_identity_sha256(),
            artifact_relative_path,
            artifact_bytes: artifact,
            outcome: observed.clone(),
            exit_code: exit,
            stdout_digest_sha256: stdout_digest,
            stderr_digest_sha256: stderr_digest,
            output_redacted,
        });
        Ok(ExecutedFixture { observed, record })
    }

    #[cfg(test)]
    pub(crate) fn set_test_pre_launch_pause(path: std::path::PathBuf, milliseconds: u64) {
        TEST_PRE_LAUNCH_PAUSED.store(false, Ordering::SeqCst);
        *pre_launch_hook()
            .lock()
            .expect("fixture pre-launch hook lock") = Some((path, milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_pre_launch_is_paused() -> bool {
        TEST_PRE_LAUNCH_PAUSED.load(Ordering::SeqCst)
    }
}

pub(crate) fn observed_from(expected: &ExpectedOutcome) -> ObservedOutcome {
    match expected.verdict {
        OutcomeVerdict::Pass => ObservedOutcome::pass(expected.maximum_claim_ceiling),
        OutcomeVerdict::Fail => {
            ObservedOutcome::failure(&expected.causal_code, expected.maximum_claim_ceiling)
        }
        OutcomeVerdict::Quarantined => ObservedOutcome::quarantined(&expected.causal_code),
    }
}
