pub(in crate::evaluation) use super::ledger::ProductionExecutionRequest;
use super::{
    CanonicalEvaluationFailure, CanonicalEvaluationRun, EvaluationError, PrivacySafeEvaluationEvent,
};
use crate::fixture_scheduler::{FixtureExecutionBinding, FixtureExecutionRecord};
use serde::{Deserialize, Serialize};
use std::fmt;

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
pub(crate) struct ProductionEvaluationRun {
    pub canonical_run: CanonicalEvaluationRun,
    pub canonical_failures: Vec<CanonicalEvaluationFailure>,
    pub events: Vec<PrivacySafeEvaluationEvent>,
    public_result: Vec<u8>,
    pub(crate) fixture_records: Vec<FixtureExecutionRecord>,
}

impl ProductionEvaluationRun {
    pub(crate) fn new(
        canonical_run: CanonicalEvaluationRun,
        canonical_failures: Vec<CanonicalEvaluationFailure>,
        events: Vec<PrivacySafeEvaluationEvent>,
        public_result: Vec<u8>,
        fixture_records: Vec<FixtureExecutionRecord>,
    ) -> Self {
        Self {
            canonical_run,
            canonical_failures,
            events,
            public_result,
            fixture_records,
        }
    }

    pub(crate) fn public_result(&self) -> &[u8] {
        &self.public_result
    }

    pub(crate) fn fixture_records(&self) -> &[FixtureExecutionRecord] {
        &self.fixture_records
    }

    pub(crate) fn from_terminal(public_result: Vec<u8>) -> Result<Self, ProductionRuntimeError> {
        #[derive(Deserialize)]
        struct TerminalResult {
            canonical_run: CanonicalEvaluationRun,
            canonical_failures: Vec<CanonicalEvaluationFailure>,
            events: Vec<PrivacySafeEvaluationEvent>,
        }
        let result = serde_json::from_slice::<TerminalResult>(&public_result)
            .map_err(|_| ProductionRuntimeError::new("evaluation-terminal-result-invalid"))?;
        Ok(Self::new(
            result.canonical_run,
            result.canonical_failures,
            result.events,
            public_result,
            Vec::new(),
        ))
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
