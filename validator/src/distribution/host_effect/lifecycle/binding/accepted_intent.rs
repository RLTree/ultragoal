pub(crate) const fn matches_accepted_intent(
    operation: AcceptedLifecycleOperation,
    intent: crate::plugin_product::lifecycle::LifecycleIntent,
) -> bool {
    use crate::plugin_product::lifecycle::LifecycleIntent as Intent;
    matches!(
        (operation, intent),
        (
            AcceptedLifecycleOperation::FreshInstall,
            Intent::FreshInstall
        ) | (
            AcceptedLifecycleOperation::MonotonicUpdate,
            Intent::MonotonicUpdate
        ) | (
            AcceptedLifecycleOperation::FailedUpdateRecovery,
            Intent::FailedUpdateRecovery
        ) | (
            AcceptedLifecycleOperation::AuthorizedRollback,
            Intent::AuthorizedRollback
        ) | (
            AcceptedLifecycleOperation::IdempotentReinstall,
            Intent::IdempotentReinstall
        ) | (
            AcceptedLifecycleOperation::UninstallTeardown,
            Intent::UninstallTeardown
        ) | (
            AcceptedLifecycleOperation::StaleCacheRecovery,
            Intent::StaleCacheRecovery
        ) | (AcceptedLifecycleOperation::RepeatUse, Intent::RepeatUse)
    )
}
