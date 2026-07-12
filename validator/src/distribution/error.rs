use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistributionErrorId {
    InvalidJson,
    InvalidSpec,
    DuplicateLayer,
    DuplicateCapability,
    InvalidPath,
    ObjectUnavailable,
    ObjectTooLarge,
    UnsafeObject,
    ObjectChanged,
    CapabilityMismatch,
    EffectFailed,
    ArchiveMismatch,
    InstallConflict,
    RollbackFailed,
    ProvenanceMismatch,
    SignaturePolicyUnavailable,
    SignatureMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DistributionError {
    id: DistributionErrorId,
}

impl DistributionError {
    pub(crate) const fn new(id: DistributionErrorId) -> Self {
        Self { id }
    }

    pub const fn id(&self) -> DistributionErrorId {
        self.id
    }
}

impl fmt::Display for DistributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.id {
            DistributionErrorId::InvalidJson => "invalid distribution JSON",
            DistributionErrorId::InvalidSpec => "invalid distribution specification",
            DistributionErrorId::DuplicateLayer => "duplicate distribution layer",
            DistributionErrorId::DuplicateCapability => "duplicate host capability",
            DistributionErrorId::InvalidPath => "invalid confined relative path",
            DistributionErrorId::ObjectUnavailable => "required object unavailable",
            DistributionErrorId::ObjectTooLarge => "bounded object limit exceeded",
            DistributionErrorId::UnsafeObject => "unsafe filesystem object",
            DistributionErrorId::ObjectChanged => "object changed during read session",
            DistributionErrorId::CapabilityMismatch => "capability and observation disagree",
            DistributionErrorId::EffectFailed => "authorized effect failed",
            DistributionErrorId::ArchiveMismatch => "package archive verification failed",
            DistributionErrorId::InstallConflict => "installed state conflicts with plan",
            DistributionErrorId::RollbackFailed => "installation rollback failed",
            DistributionErrorId::ProvenanceMismatch => "provenance expectation mismatch",
            DistributionErrorId::SignaturePolicyUnavailable => "signature policy unavailable",
            DistributionErrorId::SignatureMismatch => "signature expectation mismatch",
        })
    }
}

impl std::error::Error for DistributionError {}

pub(crate) fn error(id: DistributionErrorId) -> DistributionError {
    DistributionError::new(id)
}
