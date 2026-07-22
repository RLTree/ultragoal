use super::executor::{
    ConfinedHostEffectTarget, HostEffectCancellation, HostEffectExecutionPolicy,
    NativeRetainedDescriptorProcessBackend,
};
use super::lifecycle::DescriptorExecutionHandoff;
use super::transaction_observation::observe;
use super::transaction_policy;
use super::transaction_recovery::ObservedRecoveryAdapter;
use super::{
    DurableHostEffectLedger, FileHostEffectLedger, HostEffectRecoveryHandoff, HostEffectState,
    HostLifecycleObservationInput,
};
use super::{HostEffectCompletion, SelectedCodexExecutable};
use crate::plugin_product::lifecycle::HostLifecycleCustody;
use std::path::PathBuf;

const MAX_REOBSERVATION_ATTEMPTS: u8 = 3;

pub(super) struct HostLifecycleAttemptIdentity {
    permit_id: String,
    effect_identity_sha256: String,
    command_plan_sha256: String,
    executable_identity_sha256: String,
    target_identity_sha256: String,
    observation_attempt: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HostLifecycleRecoveryCause {
    Execution(&'static str),
    Observation(&'static str),
    Settlement(&'static str),
    Finalization(&'static str),
}

pub(crate) struct HostLifecycleRecoveryCarrier {
    handoff: DescriptorExecutionHandoff,
    custody: HostLifecycleCustody,
    ledger: FileHostEffectLedger,
    observation_executable: super::SelectedCodexExecutable,
    observation_target: ConfinedHostEffectTarget,
    observation_input: HostLifecycleObservationInput,
    target_root: PathBuf,
    policy: HostEffectExecutionPolicy,
    environment: Vec<(String, String)>,
    recovery: Option<HostEffectRecoveryHandoff>,
    command_output_sha256: Vec<String>,
    cause: HostLifecycleRecoveryCause,
    attempt: HostLifecycleAttemptIdentity,
}

impl HostLifecycleRecoveryCarrier {
    pub(super) fn from_execution(
        handoff: DescriptorExecutionHandoff,
        custody: HostLifecycleCustody,
        ledger: FileHostEffectLedger,
        observation_executable: SelectedCodexExecutable,
        observation_target: ConfinedHostEffectTarget,
        observation_input: HostLifecycleObservationInput,
        target_root: PathBuf,
        policy: HostEffectExecutionPolicy,
        environment: Vec<(String, String)>,
        recovery: Option<HostEffectRecoveryHandoff>,
        command_output_sha256: Vec<String>,
        cause: HostLifecycleRecoveryCause,
    ) -> Self {
        let (permit_id, command_plan_sha256, executable_identity_sha256, target_identity_sha256) =
            handoff.recovery_bindings();
        let effect_identity_sha256 = recovery
            .as_ref()
            .map(|row| row.effect_identity_sha256().to_owned())
            .unwrap_or_default();
        let command_output_sha256 = if command_output_sha256.is_empty() {
            recovery
                .as_ref()
                .and_then(|row| row.outcome())
                .map(|outcome| outcome.command_output_sha256().to_vec())
                .unwrap_or(command_output_sha256)
        } else {
            command_output_sha256
        };
        Self {
            handoff,
            custody,
            ledger,
            observation_executable,
            observation_target,
            observation_input,
            target_root,
            policy,
            environment,
            recovery,
            command_output_sha256,
            cause,
            attempt: HostLifecycleAttemptIdentity {
                permit_id,
                effect_identity_sha256,
                command_plan_sha256,
                executable_identity_sha256,
                target_identity_sha256,
                observation_attempt: 0,
            },
        }
    }
}

pub(crate) enum HostLifecycleTransactionOutcome {
    Completed(super::transaction::HostLifecycleTransactionResult),
    FinalizedFailure(&'static str),
    RecoveryRequired(HostLifecycleRecoveryCarrier),
}

include!("transaction_carrier_reobservation.rs");
