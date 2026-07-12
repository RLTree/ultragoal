use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FitErrorId {
    InvalidSpec,
    InvalidPath,
    ResourceLimit,
    UnsafeObject,
    ReadFailed,
    Conflict,
    StaleBinding,
    Unauthorized,
    EffectFailed,
    RollbackFailed,
    VerificationFailed,
    UnsupportedHost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct FitError {
    id: FitErrorId,
}

impl FitError {
    pub const fn id(&self) -> FitErrorId {
        self.id
    }
}

impl fmt::Display for FitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "repository fit failed: {}", code(self.id))
    }
}

impl std::error::Error for FitError {}

pub(crate) const fn error(id: FitErrorId) -> FitError {
    FitError { id }
}

const fn code(id: FitErrorId) -> &'static str {
    match id {
        FitErrorId::InvalidSpec => "HUFIT-001",
        FitErrorId::InvalidPath => "HUFIT-002",
        FitErrorId::ResourceLimit => "HUFIT-003",
        FitErrorId::UnsafeObject => "HUFIT-004",
        FitErrorId::ReadFailed => "HUFIT-005",
        FitErrorId::Conflict => "HUFIT-006",
        FitErrorId::StaleBinding => "HUFIT-007",
        FitErrorId::Unauthorized => "HUFIT-008",
        FitErrorId::EffectFailed => "HUFIT-009",
        FitErrorId::RollbackFailed => "HUFIT-010",
        FitErrorId::VerificationFailed => "HUFIT-011",
        FitErrorId::UnsupportedHost => "HUFIT-012",
    }
}
