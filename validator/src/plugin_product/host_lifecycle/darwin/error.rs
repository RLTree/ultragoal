use super::DarwinHostSurface;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DarwinHostErrorId {
    UnsupportedPlatform,
    InvalidPackage,
    UnsupportedVersion,
    InvalidMarketplace,
    InvalidOperation,
    ConfinementRejected,
    UnsafeObject,
    ObjectTooLarge,
    ObservationChanged,
    SurfaceConflict,
    CrossSurfaceSubstitution,
    PackageSubstitution,
    JournalConflict,
    JournalCorrupt,
    LineageConflict,
    LineageCorrupt,
    RecoveryRequired,
    ConcurrentTransaction,
    EffectFailed,
    Interrupted,
    CancellationUnsafe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DarwinHostError {
    id: DarwinHostErrorId,
    surface: Option<DarwinHostSurface>,
}

impl DarwinHostError {
    pub(super) const fn new(id: DarwinHostErrorId) -> Self {
        Self { id, surface: None }
    }

    pub(super) const fn at(id: DarwinHostErrorId, surface: DarwinHostSurface) -> Self {
        Self {
            id,
            surface: Some(surface),
        }
    }

    pub const fn id(&self) -> DarwinHostErrorId {
        self.id
    }

    pub const fn surface(&self) -> Option<DarwinHostSurface> {
        self.surface
    }
}

impl fmt::Display for DarwinHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.id {
            DarwinHostErrorId::UnsupportedPlatform => "Darwin host transactions are unavailable",
            DarwinHostErrorId::InvalidPackage => "supported package binding is invalid",
            DarwinHostErrorId::UnsupportedVersion => "package version is not supported",
            DarwinHostErrorId::InvalidMarketplace => "marketplace binding is invalid",
            DarwinHostErrorId::InvalidOperation => "host transaction operation is invalid",
            DarwinHostErrorId::ConfinementRejected => "host transaction root is not confined",
            DarwinHostErrorId::UnsafeObject => "host surface contains an unsafe object",
            DarwinHostErrorId::ObjectTooLarge => "host surface exceeds its bounded limit",
            DarwinHostErrorId::ObservationChanged => "host surface changed during the transaction",
            DarwinHostErrorId::SurfaceConflict => "host surface conflicts with the transaction",
            DarwinHostErrorId::CrossSurfaceSubstitution => {
                "host surface contains a record issued for another surface"
            }
            DarwinHostErrorId::PackageSubstitution => {
                "host surface contains a different package binding"
            }
            DarwinHostErrorId::JournalConflict => {
                "host transaction journal conflicts with the plan"
            }
            DarwinHostErrorId::JournalCorrupt => "host transaction journal is invalid",
            DarwinHostErrorId::LineageConflict => {
                "host transaction lineage conflicts with durable authority"
            }
            DarwinHostErrorId::LineageCorrupt => "host transaction lineage is invalid",
            DarwinHostErrorId::RecoveryRequired => "host transaction recovery is required",
            DarwinHostErrorId::ConcurrentTransaction => {
                "another host transaction owns the confined root"
            }
            DarwinHostErrorId::EffectFailed => "confined host effect failed",
            DarwinHostErrorId::Interrupted => "host transaction was interrupted",
            DarwinHostErrorId::CancellationUnsafe => {
                "host transaction cannot be cancelled after an effect"
            }
        };
        if let Some(surface) = self.surface {
            write!(formatter, "{message}: {surface:?}")
        } else {
            formatter.write_str(message)
        }
    }
}

impl std::error::Error for DarwinHostError {}
