use super::*;
use crate::routine_work::runtime_adapter::production::{
    ReservedRecoveryEffectEvidence, ReservedRecoveryOwnerObservation,
    RoutineReservedRecoveryAssessment,
};
use std::mem::MaybeUninit;
use std::path::Path;

impl FileLedger {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn assess_reserved_recovery(
        &self,
        local: &mut LocalHead,
        target: &Path,
        context_id: &str,
        candidate_id: &str,
        plan_id: &str,
        snapshot_id: &str,
        continuation: &str,
        recovery_marker: &str,
        predecessor_continuations: &[String],
        attempt_grant: &str,
        authenticated_ledger_head: &str,
    ) -> Result<RoutineReservedRecoveryAssessment, RoutineError> {
        if !super::checkpoint_attestation::admissible_target(target, "reserved", None, false)
            || !valid(context_id)
            || !valid(candidate_id)
            || !valid(plan_id)
            || !valid(snapshot_id)
            || !super::checkpoint_attestation::valid_continuation(continuation)
            || !valid(recovery_marker)
            || !valid(attempt_grant)
            || !valid(authenticated_ledger_head)
        {
            return Err(error(
                "routine-production-reserved-assessment-attestation-invalid",
            ));
        }

        self.transition_payload(local, PublicationContext::read(), |payload, _tick, head| {
            if head == authenticated_ledger_head {
                return Err(error("routine-production-reserved-assessment-head-current"));
            }
            let record = payload
                .attempts
                .get(attempt_grant)
                .ok_or_else(|| error("routine-production-checkpoint-attempt-missing"))?;
            if record.binding.context_id != context_id
                || record.binding.candidate_id != candidate_id
                || record.binding.plan_id != plan_id
                || record.binding.snapshot_id != snapshot_id
                || record.recovery_marker != recovery_marker
                || record.predecessor_continuations != predecessor_continuations
                || super::checkpoint_attestation::continuation_for(record) != continuation
            {
                return Err(error("routine-production-checkpoint-binding-invalid"));
            }
            if record.state != AttemptState::Reserved {
                return Err(error("routine-production-checkpoint-state-invalid"));
            }

            let effect = if pristine_no_effect_record(record) {
                ReservedRecoveryEffectEvidence::PristineNoEffect
            } else {
                ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent
            };
            let owner = if effect == ReservedRecoveryEffectEvidence::PristineNoEffect {
                observe_owner(&record.owner)
            } else {
                ReservedRecoveryOwnerObservation::Unavailable
            };
            Ok((RoutineReservedRecoveryAssessment { owner, effect }, false))
        })?
        .into_result()
    }
}

fn pristine_no_effect_record(record: &ProtocolRecord) -> bool {
    record.child.is_none()
        && record.launch_stage.is_none()
        && record.next_intent == 0
        && record
            .output_journal
            .components
            .iter()
            .all(|component| component.staged.is_none() && component.provisioned.is_none())
        && record.terminal.is_none()
        && record.publication_ambiguity.is_none()
        && record.failure_evidence.is_empty()
}

fn observe_owner(owner: &OwnerLease) -> ReservedRecoveryOwnerObservation {
    let mut info = MaybeUninit::<libc::proc_bsdinfo>::zeroed();
    let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: `__error` returns thread-local errno storage for this process.
    unsafe { *libc::__error() = 0 };
    // SAFETY: `info` is correctly sized writable `proc_bsdinfo` storage and the
    // stored positive process identifier was authenticated from the private ledger.
    let observed = unsafe {
        libc::proc_pidinfo(
            owner.process_id,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if observed == 0 {
        // SAFETY: reading thread-local errno immediately after `proc_pidinfo`
        // classifies a missing process separately from an unavailable observation.
        return match unsafe { *libc::__error() } {
            libc::ESRCH => ReservedRecoveryOwnerObservation::NotObserved,
            _ => ReservedRecoveryOwnerObservation::Unavailable,
        };
    }
    if observed != size {
        return ReservedRecoveryOwnerObservation::Unavailable;
    }
    // SAFETY: an exact returned size proves that `proc_pidinfo` initialized `info`.
    let info = unsafe { info.assume_init() };
    if info.pbi_start_tvsec == owner.start_seconds
        && info.pbi_start_tvusec == owner.start_microseconds
    {
        ReservedRecoveryOwnerObservation::Active
    } else {
        ReservedRecoveryOwnerObservation::NotObserved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_owner() -> OwnerLease {
        let process_id = std::process::id() as i32;
        let mut info = MaybeUninit::<libc::proc_bsdinfo>::zeroed();
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
        // SAFETY: `info` is correctly sized writable storage for the current process.
        let observed = unsafe {
            libc::proc_pidinfo(
                process_id,
                libc::PROC_PIDTBSDINFO,
                0,
                info.as_mut_ptr().cast(),
                size,
            )
        };
        assert_eq!(observed, size);
        // SAFETY: the exact returned size proves initialization.
        let info = unsafe { info.assume_init() };
        OwnerLease {
            process_id,
            start_seconds: info.pbi_start_tvsec,
            start_microseconds: info.pbi_start_tvusec,
            nonce_sha256: format!("sha256:{}", "a".repeat(64)),
        }
    }

    #[test]
    fn owner_observation_distinguishes_the_exact_process_from_pid_reuse() {
        let owner = current_owner();
        assert_eq!(
            observe_owner(&owner),
            ReservedRecoveryOwnerObservation::Active
        );

        let mut replaced = owner;
        replaced.start_microseconds = replaced.start_microseconds.wrapping_add(1);
        assert_eq!(
            observe_owner(&replaced),
            ReservedRecoveryOwnerObservation::NotObserved
        );
    }
}
