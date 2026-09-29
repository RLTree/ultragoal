pub struct FixtureExecutionRecordCapture {
    pub binding: FixtureExecutionBinding,
    pub fixture_id: String,
    pub fixture_digest_sha256: String,
    pub lease_id: String,
    pub executable_digest_sha256: String,
    pub executable_identity_sha256: String,
    pub artifact_relative_path: String,
    pub artifact_bytes: Vec<u8>,
    pub outcome: ObservedOutcome,
    pub exit_code: Option<i32>,
    pub stdout_digest_sha256: String,
    pub stderr_digest_sha256: String,
    pub output_redacted: bool,
}

impl FixtureExecutionRecord {
    pub(crate) fn captured(capture: FixtureExecutionRecordCapture) -> Self {
        let FixtureExecutionRecordCapture {
            binding,
            fixture_id,
            fixture_digest_sha256,
            lease_id,
            executable_digest_sha256,
            executable_identity_sha256,
            artifact_relative_path,
            artifact_bytes,
            outcome,
            exit_code,
            stdout_digest_sha256,
            stderr_digest_sha256,
            output_redacted,
        } = capture;
        let artifact_digest_sha256 = digest(&artifact_bytes);
        let artifact_byte_length = artifact_bytes.len() as u64;
        let record_sha256 = digest(
            format!(
                "FixtureExecutionRecord-v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{}|{}|{}",
                binding.live_context_id,
                binding.candidate_id,
                binding.spec_sha256,
                binding.task_id,
                binding.execution_session_id,
                fixture_id,
                fixture_digest_sha256,
                lease_id,
                executable_digest_sha256,
                executable_identity_sha256,
                artifact_relative_path,
                artifact_digest_sha256,
                artifact_byte_length,
                outcome,
                exit_code,
                stdout_digest_sha256,
                stderr_digest_sha256,
                output_redacted,
            )
            .as_bytes(),
        );
        Self {
            schema_version: "FixtureExecutionRecord-v1",
            binding,
            fixture_id,
            fixture_digest_sha256,
            lease_id,
            executable_digest_sha256,
            executable_identity_sha256,
            artifact_relative_path,
            artifact_digest_sha256,
            artifact_byte_length,
            artifact_bytes,
            outcome,
            exit_code,
            stdout_digest_sha256,
            stderr_digest_sha256,
            output_redacted,
            record_sha256,
        }
    }

    pub fn record_sha256(&self) -> &str {
        &self.record_sha256
    }

    #[cfg(test)]
    pub(crate) fn artifact_bytes(&self) -> &[u8] {
        &self.artifact_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutedFixture {
    pub observed: ObservedOutcome,
    pub record: FixtureExecutionRecord,
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
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

impl ObservedOutcome {
    pub fn pass(claim_ceiling: u8) -> Self {
        Self {
            verdict: OutcomeVerdict::Pass,
            causal_code: "behavioral-pass".to_owned(),
            claim_ceiling,
        }
    }

    pub fn failure(code: impl Into<String>, claim_ceiling: u8) -> Self {
        Self {
            verdict: OutcomeVerdict::Fail,
            causal_code: code.into(),
            claim_ceiling,
        }
    }

    pub fn quarantined(code: impl Into<String>) -> Self {
        Self {
            verdict: OutcomeVerdict::Quarantined,
            causal_code: code.into(),
            claim_ceiling: 0,
        }
    }

    pub(crate) fn matches(&self, expected: &ExpectedOutcome) -> Result<(), FixtureScheduleError> {
        if self.verdict != expected.verdict || self.causal_code != expected.causal_code {
            return Err(FixtureScheduleError::ExpectationMismatch {
                expected: format!("{}:{}", expected.verdict.label(), expected.causal_code),
                observed: format!("{}:{}", self.verdict.label(), self.causal_code),
            });
        }
        if self.claim_ceiling > expected.maximum_claim_ceiling {
            return Err(FixtureScheduleError::ExpectationMismatch {
                expected: format!("claim ceiling <= {}", expected.maximum_claim_ceiling),
                observed: format!("claim ceiling {}", self.claim_ceiling),
            });
        }
        Ok(())
    }
}
