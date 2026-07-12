use super::{ConfinementPolicy, ExpectedOutcome, FixtureScheduleError};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FixtureKind {
    Positive,
    Negative,
    Tamper,
    Stale,
    Recovery,
    RewardHacking,
}

impl FixtureKind {
    fn label(&self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Tamper => "tamper",
            Self::Stale => "stale",
            Self::Recovery => "recovery",
            Self::RewardHacking => "reward-hacking",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResourceKind {
    File,
    Env,
    Process,
    Port,
    Cache,
    Telemetry,
    Install,
}

impl ResourceKind {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Env => "env",
            Self::Process => "process",
            Self::Port => "port",
            Self::Cache => "cache",
            Self::Telemetry => "telemetry",
            Self::Install => "install",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureSpec {
    pub id: String,
    pub kind: FixtureKind,
    pub semantic_target: String,
    pub resources: BTreeSet<ResourceKind>,
    pub expected: ExpectedOutcome,
    pub flaky: bool,
    pub confinement: ConfinementPolicy,
    pub metadata_digest: String,
}

impl FixtureSpec {
    pub fn new(
        id: impl Into<String>,
        kind: FixtureKind,
        semantic_target: impl Into<String>,
        resources: BTreeSet<ResourceKind>,
        expected: ExpectedOutcome,
        flaky: bool,
    ) -> Result<Self, FixtureScheduleError> {
        Self::new_with_confinement(
            id,
            kind,
            semantic_target,
            resources,
            expected,
            flaky,
            ConfinementPolicy::strict(),
        )
    }

    pub fn new_with_confinement(
        id: impl Into<String>,
        kind: FixtureKind,
        semantic_target: impl Into<String>,
        resources: BTreeSet<ResourceKind>,
        expected: ExpectedOutcome,
        flaky: bool,
        confinement: ConfinementPolicy,
    ) -> Result<Self, FixtureScheduleError> {
        let mut value = Self {
            id: id.into(),
            kind,
            semantic_target: semantic_target.into(),
            resources,
            expected,
            flaky,
            confinement,
            metadata_digest: String::new(),
        };
        value.metadata_digest = value.calculated_digest();
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), FixtureScheduleError> {
        if !valid_identifier(&self.id)
            || self.semantic_target.is_empty()
            || self.semantic_target.len() > 240
            || self.resources.is_empty()
        {
            return Err(FixtureScheduleError::InvalidMetadata(self.id.clone()));
        }
        self.expected.validate()?;
        self.confinement.validate()?;
        if self.metadata_digest != self.calculated_digest() {
            return Err(FixtureScheduleError::Integrity(self.id.clone()));
        }
        if self.kind != FixtureKind::Positive
            && self.expected.verdict != super::OutcomeVerdict::Fail
        {
            return Err(FixtureScheduleError::InvalidMetadata(
                "proof controls must expect a causal failure".to_owned(),
            ));
        }
        Ok(())
    }

    pub(crate) fn calculated_digest(&self) -> String {
        let resources = self
            .resources
            .iter()
            .map(ResourceKind::label)
            .collect::<Vec<_>>()
            .join(",");
        stable_digest(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.id,
            self.kind.label(),
            self.semantic_target,
            resources,
            self.expected.verdict.label(),
            self.expected.causal_code,
            self.expected.maximum_claim_ceiling,
            self.confinement.digest_fragment()
        ))
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

pub(crate) fn stable_digest(value: &str) -> String {
    let mut state = 0xcbf29ce484222325_u64;
    for byte in value.bytes() {
        state = (state ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{state:016x}")
}
