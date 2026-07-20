use crate::distribution::host_effect::{
    DurableHostEffectLedger, HostEffectLedgerError, HostEffectLedgerErrorId,
    HostEffectLedgerRecord, HostEffectPermit, HostEffectReservation, HostEffectState,
    HostEffectTransition, VerifiedHostEffectPermit,
};
use crate::plugin_product::lifecycle::{
    HostEffectExecutionBinding, HostLifecycleObservedBundle, HostLifecycleRecord, LifecycleEffect,
    LifecycleState,
};
use sha2::{Digest, Sha256};

/// Evidence that the root host transaction created a durable in-flight ledger
/// record before a transferred lifecycle plan may enter Applying.
pub(crate) struct DurableHostLifecycleAdmission {
    record: HostLifecycleRecord,
    _in_flight: HostEffectLedgerRecord,
    #[cfg(not(test))]
    permit: Option<HostEffectPermit>,
}

impl DurableHostLifecycleAdmission {
    fn from_transition(
        record: HostLifecycleRecord,
        reserved: HostEffectLedgerRecord,
        in_flight: HostEffectLedgerRecord,
        permit: HostEffectPermit,
    ) -> Result<Self, HostEffectLedgerError> {
        let reservation = in_flight.reservation();
        let digest_matches = reservation.lifecycle_record_sha256().is_some_and(|digest| {
            serde_json::to_vec(&record)
                .ok()
                .is_some_and(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)) == digest)
        });
        if record.validate().is_err()
            || reserved.state() != HostEffectState::Reserved
            || in_flight.state() != HostEffectState::InFlight
            || reserved.reservation() != reservation
            || reservation.lifecycle_record() != Some(&record)
            || !digest_matches
            || reservation.expected_head_sha256() != reserved.prior_head().head_sha256()
            || reserved.current_head() != in_flight.prior_head()
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        #[cfg(test)]
        let _ = permit;
        Ok(Self {
            record,
            _in_flight: in_flight,
            #[cfg(not(test))]
            permit: Some(permit),
        })
    }

    pub(crate) fn record(&self) -> &HostLifecycleRecord {
        &self.record
    }

    #[cfg(not(test))]
    pub(crate) fn take_effect_parts(
        mut self,
    ) -> Result<(HostEffectPermit, HostEffectLedgerRecord), HostEffectLedgerError> {
        let permit = self
            .permit
            .take()
            .ok_or_else(|| HostEffectLedgerError::new(HostEffectLedgerErrorId::Replay))?;
        Ok((permit, self._in_flight))
    }
}

pub(crate) fn reserve_in_flight_lifecycle(
    ledger: &dyn DurableHostEffectLedger,
    permit: VerifiedHostEffectPermit,
    record: HostLifecycleRecord,
) -> Result<DurableHostLifecycleAdmission, HostEffectLedgerError> {
    if permit.binding().lifecycle_record.as_ref() != Some(&record) {
        return Err(HostEffectLedgerError::new(
            HostEffectLedgerErrorId::InvalidRecord,
        ));
    }
    let permit = permit.into_permit();
    let reservation = HostEffectReservation::from_permit(&permit);
    let reserved = ledger.reserve(reservation)?;
    let in_flight = ledger.transition(HostEffectTransition::new(
        reserved.reservation().permit_id().to_owned(),
        HostEffectState::Reserved,
        HostEffectState::InFlight,
        reserved.current_head().clone(),
        None,
    )?)?;
    DurableHostLifecycleAdmission::from_transition(record, reserved, in_flight, permit)
}

/// A host-effect executor terminal observation for one transferred plan.
/// Only this executor leaf can mint a terminal outcome; plugin custody can
/// validate it but cannot mint success or recovery authority for itself.
pub(crate) struct HostEffectCompletion {
    binding: HostEffectExecutionBinding,
    outcome: HostEffectCompletionOutcome,
}

pub(crate) enum HostEffectCompletionOutcome {
    Settled {
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
        observations: HostLifecycleObservedBundle,
    },
    Ambiguous {
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
        observations: HostLifecycleObservedBundle,
    },
}

impl HostEffectCompletion {
    fn settled(
        binding: HostEffectExecutionBinding,
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
        observations: HostLifecycleObservedBundle,
    ) -> Self {
        Self {
            binding,
            outcome: HostEffectCompletionOutcome::Settled {
                observed,
                completed_effects,
                observations,
            },
        }
    }

    fn ambiguous(
        binding: HostEffectExecutionBinding,
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
        observations: HostLifecycleObservedBundle,
    ) -> Self {
        Self {
            binding,
            outcome: HostEffectCompletionOutcome::Ambiguous {
                observed,
                completed_effects,
                observations,
            },
        }
    }

    pub(crate) fn binding(&self) -> &HostLifecycleRecord {
        self.binding.record()
    }

    pub(crate) fn outcome(&self) -> &HostEffectCompletionOutcome {
        &self.outcome
    }
}

#[cfg(test)]
#[path = "lifecycle_completion_tests.rs"]
mod tests;
