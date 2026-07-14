use crate::distribution::DistributionError;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterErrorId {
    DistributionEffect,
    EffectOrder,
    InvalidLifecycleBinding,
    InvalidPackageBinding,
    PhysicalStateMismatch,
    RecoveryRefused,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdapterError {
    id: AdapterErrorId,
}

impl AdapterError {
    pub const fn id(self) -> AdapterErrorId {
        self.id
    }

    pub(super) const fn new(id: AdapterErrorId) -> Self {
        Self { id }
    }

    pub(super) const fn distribution(_: DistributionError) -> Self {
        Self::new(AdapterErrorId::DistributionEffect)
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.id {
            AdapterErrorId::DistributionEffect => "plugin adapter distribution effect failed",
            AdapterErrorId::EffectOrder => "plugin adapter effect order refused",
            AdapterErrorId::InvalidLifecycleBinding => "plugin lifecycle binding refused",
            AdapterErrorId::InvalidPackageBinding => "plugin package binding refused",
            AdapterErrorId::PhysicalStateMismatch => "plugin adapter state mismatch",
            AdapterErrorId::RecoveryRefused => "plugin adapter recovery refused",
        })
    }
}

impl std::error::Error for AdapterError {}
