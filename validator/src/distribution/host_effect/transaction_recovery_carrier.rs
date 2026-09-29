use super::HostEffectRecoveryHandoff;
use super::transaction_carrier::{
    HostLifecycleRecoveryCarrier, HostLifecycleRecoveryCause, HostLifecycleTransactionOutcome,
};
use super::transaction_observation::HostLifecycleObservationInput;
use std::path::Path;

pub(super) fn recovery_required(
    handoff: super::lifecycle::DescriptorExecutionHandoff,
    custody: crate::plugin_product::lifecycle::HostLifecycleCustody,
    ledger: super::FileHostEffectLedger,
    observation_target: super::ConfinedHostEffectTarget,
    observation: HostLifecycleObservationInput,
    target_root: &Path,
    policy: super::HostEffectExecutionPolicy,
    environment: Vec<(String, String)>,
    recovery: Option<HostEffectRecoveryHandoff>,
    command_output_sha256: Vec<String>,
    cause: HostLifecycleRecoveryCause,
) -> HostLifecycleTransactionOutcome {
    HostLifecycleTransactionOutcome::RecoveryRequired(HostLifecycleRecoveryCarrier::from_execution(
        handoff,
        custody,
        ledger,
        observation_target,
        observation,
        target_root.to_path_buf(),
        policy,
        environment,
        recovery,
        command_output_sha256,
        cause,
    ))
}
