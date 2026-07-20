use crate::distribution::host_effect::{HostEffectLedgerRecord, HostEffectState};
use crate::plugin_product::lifecycle::{
    HostEffectExecutionBinding, HostLifecycleRecord, LifecycleEffect, LifecycleState,
};

/// Evidence that the root host transaction created a durable in-flight ledger
/// record before a transferred lifecycle plan may enter Applying.
pub(crate) struct DurableHostLifecycleAdmission {
    record: HostLifecycleRecord,
    _ledger: Option<HostEffectLedgerRecord>,
}

impl DurableHostLifecycleAdmission {
    pub(in crate::distribution::host_effect) fn in_flight(
        record: HostLifecycleRecord,
        ledger: HostEffectLedgerRecord,
    ) -> Option<Self> {
        (ledger.state() == HostEffectState::InFlight
            && ledger.reservation().lifecycle_record() == Some(&record)
            && ledger.reservation().expected_head_sha256() == ledger.prior_head().head_sha256())
        .then_some(Self {
            record,
            _ledger: Some(ledger),
        })
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn test_record(record: HostLifecycleRecord) -> Self {
        Self {
            record,
            _ledger: None,
        }
    }

    pub(crate) fn record(&self) -> &HostLifecycleRecord {
        &self.record
    }
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
    },
    Ambiguous {
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
    },
}

impl HostEffectCompletion {
    fn settled(
        binding: HostEffectExecutionBinding,
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
    ) -> Self {
        Self {
            binding,
            outcome: HostEffectCompletionOutcome::Settled {
                observed,
                completed_effects,
            },
        }
    }

    fn ambiguous(
        binding: HostEffectExecutionBinding,
        observed: LifecycleState,
        completed_effects: Vec<LifecycleEffect>,
    ) -> Self {
        Self {
            binding,
            outcome: HostEffectCompletionOutcome::Ambiguous {
                observed,
                completed_effects,
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
mod tests {
    use super::*;
    use crate::plugin_product::lifecycle::{
        HostLifecycleCustody, LifecycleAuthorization, LifecycleIntent, LifecycleRequest,
        LifecycleState, PackageAuthority, Version, plan,
    };

    #[test]
    fn only_exact_executor_completion_can_settle_transferred_custody() {
        let before = state('a');
        let mut custody = custody(&before, false);
        let binding = begin(&mut custody);
        let expected = custody.expected_after().clone();
        let completed = custody.effects().to_vec();

        custody
            .settle(HostEffectCompletion::settled(binding, expected, completed))
            .unwrap();
    }

    #[test]
    fn completion_for_another_issued_custody_cannot_settle_the_same_plan() {
        let before = state('a');
        let mut custody = custody(&before, false);
        let mut other = custody(&before, false);
        let foreign = begin(&mut other);
        let expected = custody.expected_after().clone();
        let completed = custody.effects().to_vec();

        begin(&mut custody);
        assert!(matches!(
            custody.settle(HostEffectCompletion::settled(foreign, expected, completed)),
            Err(crate::plugin_product::lifecycle::LifecycleError::InvalidTransition)
        ));
    }

    #[test]
    fn ambiguous_executor_observation_arms_recovery_without_callers_choosing_it() {
        let before = state('a');
        let mut custody = custody(&before, false);
        let binding = begin(&mut custody);
        let mut observed = custody.expected_after().clone();
        observed.cache = custody.before().cache.clone();
        observed.recovery_required = true;
        let completed = custody.effects()[..1].to_vec();

        custody
            .settle(HostEffectCompletion::ambiguous(
                binding, observed, completed,
            ))
            .unwrap();
        assert!(custody.recovery_token().is_ok());
    }

    #[test]
    fn durable_admission_rejects_a_ledger_for_a_different_record() {
        let record = custody(&state('a'), false).pre_effect_record().clone();
        let other = custody(&state('b'), false).pre_effect_record().clone();
        let ledger = HostEffectLedgerRecord {
            reservation: crate::distribution::host_effect::HostEffectReservation {
                issuer_id: "fixture-root".to_owned(),
                ledger_id: "fixture-ledger".to_owned(),
                key_id: digest('1'),
                permit_id: digest('2'),
                semantic_key_sha256: digest('3'),
                nonce_sha256: digest('4'),
                binding_sha256: digest('5'),
                expected_head_sha256: digest('6'),
                issued_at_unix_ms: 1,
                expires_at_unix_ms: 2,
                lifecycle_record: Some(record.clone()),
            },
            state: HostEffectState::InFlight,
            record_sha256: digest('7'),
            prior_head: crate::distribution::host_effect::HostEffectLedgerHead::new(1, digest('6'))
                .unwrap(),
            current_head: crate::distribution::host_effect::HostEffectLedgerHead::new(
                2,
                digest('8'),
            )
            .unwrap(),
            outcome_sha256: None,
        };
        assert!(DurableHostLifecycleAdmission::in_flight(record, ledger.clone()).is_some());
        assert!(DurableHostLifecycleAdmission::in_flight(other, ledger).is_none());
    }

    fn custody(before: &LifecycleState, allow_downgrade: bool) -> HostLifecycleCustody {
        let plan = plan(
            before,
            &LifecycleRequest {
                intent: LifecycleIntent::MonotonicUpdate,
                target: Some(authority('b', "1.0.1")),
                prior_authority: None,
                authorization: LifecycleAuthorization {
                    allow_host_write: true,
                    allow_downgrade,
                    expected_installed_sha256: Some(digest('a')),
                },
            },
        )
        .unwrap();
        HostLifecycleCustody::take(plan).unwrap()
    }

    fn begin(custody: &mut HostLifecycleCustody) -> HostEffectExecutionBinding {
        let admission =
            DurableHostLifecycleAdmission::test_record(custody.pre_effect_record().clone());
        custody.begin_effects(admission).unwrap()
    }

    fn state(seed: char) -> LifecycleState {
        LifecycleState {
            installed: Some(authority(seed, "1.0.0")),
            cache: Some(authority(seed, "1.0.0")),
            generation: 1,
            recovery_required: false,
        }
    }

    fn authority(seed: char, version: &str) -> PackageAuthority {
        PackageAuthority {
            version: Version::parse(version).unwrap(),
            package_sha256: digest(seed),
            inventory_sha256: digest('c'),
            candidate_id: digest('d'),
        }
    }

    fn digest(seed: char) -> String {
        format!("sha256:{}", seed.to_string().repeat(64))
    }
}
