use super::SemanticEvent;
use super::limits::{DEFAULT_RESULTS, HARD_MAX_RESULTS};
use super::privacy;
use crate::context::LiveContext;

#[derive(Clone, Debug)]
pub struct EventQuery {
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) source_id: String,
    pub(super) operation: Option<String>,
    pub(super) outcome: Option<String>,
    pub(super) since_unix_ms: Option<u64>,
    pub(super) until_unix_ms: Option<u64>,
    pub(super) limit: usize,
}

impl EventQuery {
    pub fn new(
        context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        source_id: impl Into<String>,
    ) -> Result<Self, String> {
        let query = Self {
            context_id: context_id.into(),
            candidate_id: candidate_id.into(),
            source_id: source_id.into(),
            operation: None,
            outcome: None,
            since_unix_ms: None,
            until_unix_ms: None,
            limit: DEFAULT_RESULTS,
        };
        query.validate()?;
        Ok(query)
    }

    pub fn for_context(
        context: &LiveContext,
        source_id: impl Into<String>,
    ) -> Result<Self, String> {
        let event = SemanticEvent::for_context(
            context,
            source_id,
            "query-binding",
            0,
            0,
            "observe.query",
            "unknown",
        )?;
        Self::new(event.context_id(), event.candidate_id(), event.source_id())
    }

    pub fn operation(mut self, operation: impl Into<String>) -> Result<Self, String> {
        let operation = operation.into();
        privacy::validate_identifier("query-operation", &operation)?;
        self.operation = Some(operation);
        Ok(self)
    }

    pub fn outcome(mut self, outcome: impl Into<String>) -> Result<Self, String> {
        let outcome = outcome.into();
        privacy::validate_identifier("query-outcome", &outcome)?;
        self.outcome = Some(outcome);
        Ok(self)
    }

    pub fn since(mut self, unix_ms: u64) -> Self {
        self.since_unix_ms = Some(unix_ms);
        self
    }

    pub fn until(mut self, unix_ms: u64) -> Result<Self, String> {
        if self.since_unix_ms.is_some_and(|since| unix_ms < since) {
            return Err("observe-query-invalid: until precedes since".to_owned());
        }
        self.until_unix_ms = Some(unix_ms);
        Ok(self)
    }

    pub fn limit(mut self, limit: usize) -> Result<Self, String> {
        if !(1..=HARD_MAX_RESULTS).contains(&limit) {
            return Err(
                "observe-query-limit: result bound is outside the supported range".to_owned(),
            );
        }
        self.limit = limit;
        Ok(self)
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        privacy::validate_identifier("context-id", &self.context_id)?;
        privacy::validate_identifier("candidate-id", &self.candidate_id)?;
        privacy::validate_identifier("source-id", &self.source_id)?;
        if self
            .until_unix_ms
            .zip(self.since_unix_ms)
            .is_some_and(|(until, since)| until < since)
        {
            return Err("observe-query-invalid: until precedes since".to_owned());
        }
        Ok(())
    }

    pub(super) fn matches(&self, event: &SemanticEvent) -> bool {
        self.context_id == event.context_id()
            && self.candidate_id == event.candidate_id()
            && self.source_id == event.source_id()
            && self
                .operation
                .as_deref()
                .is_none_or(|value| value == event.operation())
            && self
                .outcome
                .as_deref()
                .is_none_or(|value| value == event.outcome())
            && self
                .since_unix_ms
                .is_none_or(|value| event.observed_at_unix_ms() >= value)
            && self
                .until_unix_ms
                .is_none_or(|value| event.observed_at_unix_ms() <= value)
    }
}
