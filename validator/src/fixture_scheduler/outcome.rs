use super::FixtureScheduleError;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservedOutcome {
    pub verdict: OutcomeVerdict,
    pub causal_code: String,
    pub claim_ceiling: u8,
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
