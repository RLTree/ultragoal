use super::effect_request::HostScopeAuthority;
use super::error::{HostLifecycleError, HostLifecycleErrorId};
use super::scope::{BoundHostScope, same_scope};
use crate::distribution::{Capability, HostCapabilityState};
use crate::plugin_product::lifecycle::LifecycleIntent;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) struct HostEffectCapabilityGate {
    intent: LifecycleIntent,
    scope: Arc<BoundHostScope>,
    required: &'static [Capability],
    gate_sha256: String,
}

impl HostEffectCapabilityGate {
    pub(super) fn issue(
        intent: LifecycleIntent,
        scope: Arc<BoundHostScope>,
    ) -> Result<Arc<Self>, HostLifecycleError> {
        let required = required_host_capabilities(intent, scope.authority());
        let gate_sha256 = gate_digest(intent, &scope, required)?;
        Ok(Arc::new(Self {
            intent,
            scope,
            required,
            gate_sha256,
        }))
    }

    pub(super) fn require_supported(&self) -> Result<(), HostLifecycleError> {
        if !capability_states_supported(self.required, |capability| {
            self.scope.host().state(capability)
        }) {
            return Err(HostLifecycleError::new(
                HostLifecycleErrorId::HostCapabilityRejected,
            ));
        }
        if !self.required.is_empty() {
            self.scope.revalidate()?;
        }
        Ok(())
    }

    pub(super) fn gate_sha256(&self) -> &str {
        &self.gate_sha256
    }

    pub(super) fn intent(&self) -> LifecycleIntent {
        self.intent
    }
}

pub(crate) fn required_host_capabilities(
    intent: LifecycleIntent,
    scope: &HostScopeAuthority,
) -> &'static [Capability] {
    const NONE: &[Capability] = &[];
    const PERSONAL: &[Capability] = &[Capability::Install, Capability::Marketplace];
    const REPOSITORY: &[Capability] = &[
        Capability::Filesystem,
        Capability::Install,
        Capability::Marketplace,
    ];
    if matches!(
        intent,
        LifecycleIntent::RepeatUse | LifecycleIntent::IdempotentReinstall
    ) {
        NONE
    } else {
        match scope {
            HostScopeAuthority::Personal { .. } => PERSONAL,
            HostScopeAuthority::Repository { .. } => REPOSITORY,
        }
    }
}

pub(crate) fn capability_states_supported(
    required: &[Capability],
    mut state: impl FnMut(Capability) -> HostCapabilityState,
) -> bool {
    required
        .iter()
        .all(|capability| state(*capability) == HostCapabilityState::Supported)
}

pub(super) fn same_gate(
    left: &Arc<HostEffectCapabilityGate>,
    right: &Arc<HostEffectCapabilityGate>,
) -> bool {
    Arc::ptr_eq(left, right)
        && left.gate_sha256 == right.gate_sha256
        && left.intent == right.intent
        && same_scope(&left.scope, &right.scope)
        && left.required == right.required
}

fn gate_digest(
    intent: LifecycleIntent,
    scope: &BoundHostScope,
    required: &[Capability],
) -> Result<String, HostLifecycleError> {
    #[derive(Serialize)]
    struct Gate<'a> {
        schema: &'static str,
        intent: LifecycleIntent,
        host_scope_sha256: &'a str,
        required: &'a [Capability],
    }
    let bytes = serde_json::to_vec(&Gate {
        schema: "harness-ultragoal.host-effect-capability-gate.v1",
        intent,
        host_scope_sha256: scope.scope_sha256(),
        required,
    })
    .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::InvalidBinding))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
