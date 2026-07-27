pub(crate) use super::production_input::ProductionExecutionRequest;
use super::{
    CanonicalEvaluationFailure, CanonicalEvaluationRun, EvaluationError, PrivacySafeEvaluationEvent,
};
use crate::fixture_scheduler::{FixtureExecutionBinding, FixtureExecutionRecord};
use serde::Serialize;
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
pub struct ProductionEvaluationRun {
    pub canonical_run: CanonicalEvaluationRun,
    pub canonical_failures: Vec<CanonicalEvaluationFailure>,
    pub events: Vec<PrivacySafeEvaluationEvent>,
    pub(crate) fixture_records: Vec<FixtureExecutionRecord>,
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
