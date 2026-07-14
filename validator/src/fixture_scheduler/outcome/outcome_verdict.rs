#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeVerdict {
    Pass,
    Fail,
    Quarantined,
}

impl OutcomeVerdict {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Quarantined => "quarantined",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedOutcome {
    pub verdict: OutcomeVerdict,
    pub causal_code: String,
    pub maximum_claim_ceiling: u8,
}

impl ExpectedOutcome {
    pub fn pass(maximum_claim_ceiling: u8) -> Self {
        Self {
            verdict: OutcomeVerdict::Pass,
            causal_code: "behavioral-pass".to_owned(),
            maximum_claim_ceiling,
        }
    }

    pub fn causal_failure(code: impl Into<String>, maximum_claim_ceiling: u8) -> Self {
        Self {
            verdict: OutcomeVerdict::Fail,
            causal_code: code.into(),
            maximum_claim_ceiling,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), FixtureScheduleError> {
        if self.causal_code.is_empty()
            || self.causal_code.len() > 120
            || self.causal_code.chars().any(char::is_control)
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "expected causal code".to_owned(),
            ));
        }
        if self.verdict == OutcomeVerdict::Quarantined {
            return Err(FixtureScheduleError::InvalidMetadata(
                "quarantine cannot be an expected pass".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObservedOutcome {
    pub verdict: OutcomeVerdict,
    pub causal_code: String,
    pub claim_ceiling: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FixtureExecutionBinding {
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub task_id: String,
    pub execution_session_id: String,
}

impl FixtureExecutionBinding {
    pub fn new(
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        spec_sha256: impl Into<String>,
        task_id: impl Into<String>,
        execution_session_id: impl Into<String>,
    ) -> Result<Self, FixtureScheduleError> {
        let value = Self {
            live_context_id: live_context_id.into(),
            candidate_id: candidate_id.into(),
            spec_sha256: spec_sha256.into(),
            task_id: task_id.into(),
            execution_session_id: execution_session_id.into(),
        };
        if !valid_sha256(&value.live_context_id)
            || !valid_sha256(&value.candidate_id)
            || !valid_sha256(&value.spec_sha256)
            || !valid_identifier(&value.task_id)
            || !valid_sha256(&value.execution_session_id)
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "fixture execution binding".to_owned(),
            ));
        }
        Ok(value)
    }

    pub(crate) fn standalone(fixture_id: &str, fixture_digest: &str) -> Self {
        let seed = digest(format!("standalone|{fixture_id}|{fixture_digest}").as_bytes());
        Self {
            live_context_id: seed.clone(),
            candidate_id: digest(format!("candidate|{seed}").as_bytes()),
            spec_sha256: digest(format!("spec|{seed}").as_bytes()),
            task_id: fixture_id.to_owned(),
            execution_session_id: digest(format!("session|{seed}").as_bytes()),
        }
    }
}

/// Bounded evidence captured while the lease still exists. Raw process output
/// is never retained. Artifact bytes are private to crate-controlled grading,
/// size-bounded, and refused when they look secret-bearing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FixtureExecutionRecord {
    pub schema_version: &'static str,
    pub binding: FixtureExecutionBinding,
    pub fixture_id: String,
    pub fixture_digest_sha256: String,
    pub lease_id: String,
    pub executable_digest_sha256: String,
    pub executable_identity_sha256: String,
    pub artifact_relative_path: String,
    pub artifact_digest_sha256: String,
    pub artifact_byte_length: u64,
    #[serde(skip)]
    artifact_bytes: Vec<u8>,
    pub outcome: ObservedOutcome,
    pub exit_code: Option<i32>,
    pub stdout_digest_sha256: String,
    pub stderr_digest_sha256: String,
    pub output_redacted: bool,
    pub record_sha256: String,
}
