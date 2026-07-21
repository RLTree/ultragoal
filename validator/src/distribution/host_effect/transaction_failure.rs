use super::executor::{HostEffectExecutorErrorId, HostEffectExecutorFailure};
use super::lifecycle::{SupportedHostLifecycleError, SupportedHostLifecycleErrorId};

pub(super) fn preparation_failure(error: SupportedHostLifecycleError) -> &'static str {
    use SupportedHostLifecycleErrorId as Id;
    match error.id() {
        Id::DescriptorExecutionUnavailable => "host lifecycle descriptor execution unavailable",
        Id::InvalidAcceptedIdentity => "host lifecycle accepted identity rejected",
        Id::CoordinatorSubstitution => "host lifecycle coordinator binding rejected",
        Id::PlanSubstitution => "host lifecycle plan binding rejected",
        Id::ExecutableSubstitution => "host lifecycle executable binding rejected",
        Id::TargetSubstitution => "host lifecycle target binding rejected",
        Id::TargetRace => "host lifecycle target changed during reservation",
        Id::UntrustedTime => "host lifecycle trusted clock unavailable",
        Id::StaleLedgerHead => "host lifecycle ledger head changed before reservation",
        Id::AuthorityRejected => "host lifecycle authority issuance rejected",
        Id::LedgerRejected => "host lifecycle durable reservation rejected",
        Id::HandoffConstructionFailed => "host lifecycle execution handoff rejected",
        #[cfg(test)]
        Id::UnsupportedPlatform => "host lifecycle platform unsupported",
        #[cfg(test)]
        Id::RecoveryAuthorizationRequired => "host lifecycle recovery authorization required",
        Id::RecoveryUnsafe => "host lifecycle recovery rejected",
    }
}

pub(super) fn execution_failure(error: HostEffectExecutorFailure) -> &'static str {
    use HostEffectExecutorErrorId as Id;
    match error.id() {
        Id::UnsupportedPlatform => "host lifecycle execution platform unsupported",
        Id::InvalidPolicy => "host lifecycle execution policy rejected",
        Id::EnvironmentInjection => "host lifecycle execution environment rejected",
        Id::InvalidTargetRoot => "host lifecycle execution target rejected",
        Id::TargetSubstitution => "host lifecycle execution target changed",
        Id::UnsafeObject => "host lifecycle execution object rejected",
        Id::PathSwap => "host lifecycle execution path changed",
        Id::ExecutableMutation => "host lifecycle executable changed",
        Id::LedgerSubstitution => "host lifecycle execution ledger changed",
        Id::LedgerRollback => "host lifecycle execution ledger rolled back",
        Id::Replay => "host lifecycle execution replay rejected",
        Id::TempCollision => "host lifecycle execution temporary collision",
        Id::RenameRace => "host lifecycle execution publication raced",
        Id::SyncFailure => "host lifecycle execution durability sync failed",
        Id::ProcessSpawnFailed => "host lifecycle process could not start",
        Id::ProcessFailed => "host lifecycle process failed",
        Id::OutputOverflow => "host lifecycle process output exceeded limit",
        Id::Timeout => "host lifecycle process timed out",
        Id::Cancelled => "host lifecycle process cancelled",
        Id::PartialAcknowledgement => "host lifecycle execution acknowledgement incomplete",
        Id::FalsePassReceipt => "host lifecycle execution receipt rejected",
        Id::RecoveryRequired => "host lifecycle execution requires recovery",
        Id::Io => "host lifecycle execution IO failed",
    }
}
