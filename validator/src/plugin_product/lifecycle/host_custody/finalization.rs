#[cfg(not(test))]
use crate::distribution::host_effect::HostEffectState;

/// A one-use proof that the transferred lifecycle reached a durable terminal
/// disposition. It is intentionally opaque outside lifecycle custody: callers
/// cannot release a staged executable before settlement, nor before an
/// ambiguous result has completed its authorized recovery.
pub(crate) struct HostLifecycleFinalization {
    #[cfg(not(test))]
    record: HostLifecycleRecord,
}

impl HostLifecycleFinalization {
    #[cfg(not(test))]
    pub(crate) fn matches(&self, binding: &HostEffectExecutionBinding) -> bool {
        &self.record == binding.record()
    }
}

/// A one-use proof that a failed or ambiguous host-effect transition was
/// durably recorded for this exact lifecycle transfer. It is issued only by
/// `HostLifecycleCustody`; callers cannot construct a release decision.
#[cfg(not(test))]
pub(crate) struct HostLifecycleRecoveryDisposition {
    record: HostLifecycleRecord,
    permit_id: String,
    effect_identity_sha256: String,
    target_identity_sha256: String,
    executable_identity_sha256: String,
    command_plan_sha256: String,
    recovery_binding_sha256: String,
    terminal_state: HostEffectState,
}

#[cfg(not(test))]
impl HostLifecycleRecoveryDisposition {
    pub(crate) fn matches(
        &self,
        binding: &HostEffectExecutionBinding,
        permit_id: &str,
        effect_identity_sha256: &str,
        target_identity_sha256: &str,
        executable_identity_sha256: &str,
        command_plan_sha256: &str,
        recovery: &HostEffectRecoveryHandoff,
    ) -> bool {
        self.record == *binding.record()
            && self.permit_id == permit_id
            && self.effect_identity_sha256 == effect_identity_sha256
            && self.target_identity_sha256 == target_identity_sha256
            && self.executable_identity_sha256 == executable_identity_sha256
            && self.command_plan_sha256 == command_plan_sha256
            && self.recovery_binding_sha256 == recovery.disposition_binding_sha256()
            && matches!(self.terminal_state, HostEffectState::Settled | HostEffectState::Failed)
            && Some(self.terminal_state) == recovery.terminal_state()
            && recovery.verify_binding()
    }
}
