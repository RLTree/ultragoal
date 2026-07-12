use super::error::{HostLifecycleError, HostLifecycleErrorId};
use crate::distribution::JourneyBinding;
use crate::plugin_product::lifecycle::LifecyclePlan;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

static NEXT_SESSION_ISSUANCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum SessionStage {
    Bound = 0,
    Applying = 1,
    EffectEligible = 2,
    RequestIssued = 3,
    RequestConsumed = 4,
    ObservationReady = 5,
    Closed = 6,
    Rejected = 7,
}

impl SessionStage {
    fn decode(value: u8) -> Result<Self, HostLifecycleError> {
        match value {
            0 => Ok(Self::Bound),
            1 => Ok(Self::Applying),
            2 => Ok(Self::EffectEligible),
            3 => Ok(Self::RequestIssued),
            4 => Ok(Self::RequestConsumed),
            5 => Ok(Self::ObservationReady),
            6 => Ok(Self::Closed),
            7 => Ok(Self::Rejected),
            _ => Err(state_error()),
        }
    }
}

pub(super) struct SessionIssuance {
    issuance_id: u64,
    issuance_sha256: String,
    lifecycle: LifecyclePlan,
    binding_sha256: String,
    stage: AtomicU8,
}

impl SessionIssuance {
    pub(super) fn issue(
        lifecycle: &LifecyclePlan,
        binding: &JourneyBinding,
    ) -> Result<Arc<Self>, HostLifecycleError> {
        let issuance_id = NEXT_SESSION_ISSUANCE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| state_error())?;
        let issuance_sha256 = issuance_digest(issuance_id, lifecycle, binding)?;
        Ok(Arc::new(Self {
            issuance_id,
            issuance_sha256,
            lifecycle: lifecycle.clone(),
            binding_sha256: binding.binding_sha256().to_owned(),
            stage: AtomicU8::new(SessionStage::Bound as u8),
        }))
    }

    pub(super) fn issuance_sha256(&self) -> &str {
        &self.issuance_sha256
    }

    pub(super) fn lifecycle(&self) -> &LifecyclePlan {
        &self.lifecycle
    }

    pub(super) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(super) fn begin_apply(&self) -> Result<(), HostLifecycleError> {
        self.transition(SessionStage::Bound, SessionStage::Applying, state_error())
    }

    pub(super) fn finish_apply(
        &self,
        applied: bool,
        has_external_effect: bool,
    ) -> Result<(), HostLifecycleError> {
        let next = if !applied {
            SessionStage::Rejected
        } else if has_external_effect {
            SessionStage::EffectEligible
        } else {
            SessionStage::ObservationReady
        };
        self.transition(SessionStage::Applying, next, state_error())
    }

    pub(super) fn reject_apply(&self) {
        self.reject();
    }

    pub(super) fn reject(&self) {
        loop {
            let observed = self.stage.load(Ordering::Acquire);
            if matches!(
                SessionStage::decode(observed),
                Ok(SessionStage::Closed | SessionStage::Rejected)
            ) {
                return;
            }
            if self
                .stage
                .compare_exchange(
                    observed,
                    SessionStage::Rejected as u8,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return;
            }
        }
    }

    pub(super) fn issue_request(&self) -> Result<(), HostLifecycleError> {
        self.transition(
            SessionStage::EffectEligible,
            SessionStage::RequestIssued,
            request_stage_error(self.stage.load(Ordering::Acquire)),
        )
    }

    pub(super) fn consume_request(&self) -> Result<(), HostLifecycleError> {
        self.transition(
            SessionStage::RequestIssued,
            SessionStage::RequestConsumed,
            request_stage_error(self.stage.load(Ordering::Acquire)),
        )
    }

    pub(super) fn release_plan(&self) -> Result<(), HostLifecycleError> {
        self.transition(
            SessionStage::RequestConsumed,
            SessionStage::ObservationReady,
            request_stage_error(self.stage.load(Ordering::Acquire)),
        )
    }

    pub(super) fn require_observation_ready(&self) -> Result<(), HostLifecycleError> {
        match SessionStage::decode(self.stage.load(Ordering::Acquire))? {
            SessionStage::ObservationReady => Ok(()),
            SessionStage::Closed | SessionStage::Rejected => Err(state_error()),
            _ => Err(HostLifecycleError::new(
                HostLifecycleErrorId::ExternalEffectNotEligible,
            )),
        }
    }

    pub(super) fn close(&self) -> Result<(), HostLifecycleError> {
        self.transition(
            SessionStage::ObservationReady,
            SessionStage::Closed,
            state_error(),
        )
    }

    fn transition(
        &self,
        expected: SessionStage,
        next: SessionStage,
        error: HostLifecycleError,
    ) -> Result<(), HostLifecycleError> {
        self.stage
            .compare_exchange(
                expected as u8,
                next as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|_| error)
    }
}

pub(super) fn same_issuance(left: &Arc<SessionIssuance>, right: &Arc<SessionIssuance>) -> bool {
    Arc::ptr_eq(left, right)
        && left.issuance_id == right.issuance_id
        && left.issuance_sha256 == right.issuance_sha256
        && left.lifecycle == right.lifecycle
        && left.binding_sha256 == right.binding_sha256
}

fn request_stage_error(stage: u8) -> HostLifecycleError {
    match SessionStage::decode(stage) {
        Ok(SessionStage::RequestIssued)
        | Ok(SessionStage::RequestConsumed)
        | Ok(SessionStage::ObservationReady)
        | Ok(SessionStage::Closed) => {
            HostLifecycleError::new(HostLifecycleErrorId::ExternalEffectReplayed)
        }
        Ok(_) => HostLifecycleError::new(HostLifecycleErrorId::ExternalEffectNotEligible),
        Err(error) => error,
    }
}

fn issuance_digest(
    issuance_id: u64,
    lifecycle: &LifecyclePlan,
    binding: &JourneyBinding,
) -> Result<String, HostLifecycleError> {
    #[derive(Serialize)]
    struct Issuance<'a> {
        schema: &'static str,
        issuance_id: u64,
        lifecycle_plan_id: &'a str,
        lifecycle_authorization_sha256: &'a str,
        binding_sha256: &'a str,
    }
    let bytes = serde_json::to_vec(&Issuance {
        schema: "harness-ultragoal.host-lifecycle-session-issuance.v1",
        issuance_id,
        lifecycle_plan_id: &lifecycle.plan_id,
        lifecycle_authorization_sha256: &lifecycle.authorization_sha256,
        binding_sha256: binding.binding_sha256(),
    })
    .map_err(|_| state_error())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn state_error() -> HostLifecycleError {
    HostLifecycleError::new(HostLifecycleErrorId::SessionStateRejected)
}
