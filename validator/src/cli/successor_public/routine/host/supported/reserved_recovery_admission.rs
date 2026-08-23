use super::super::RoutineReservedRecoveryAdmission;
use crate::routine_work::{
    ReservedRecoveryEffectEvidence, ReservedRecoveryOwnerObservation,
    RoutineReservedRecoveryAssessment,
};

pub(super) fn admission(
    assessment: RoutineReservedRecoveryAssessment,
) -> (RoutineReservedRecoveryAdmission, &'static str) {
    let (status, owner_observation, next_action) = match (assessment.effect, assessment.owner) {
        (
            ReservedRecoveryEffectEvidence::PristineNoEffect,
            ReservedRecoveryOwnerObservation::Active,
        ) => (
            "stale_reserved_active_owner_preserve_and_retry",
            "exact_owner_process_observed",
            "preserve all bytes and retry diagnosis after the exact reservation owner exits",
        ),
        (
            ReservedRecoveryEffectEvidence::PristineNoEffect,
            ReservedRecoveryOwnerObservation::NotObserved,
        ) => (
            "stale_reserved_abandoned_candidate_preserve_and_hold",
            "owner_process_not_observed",
            "preserve all bytes and independently review a current-head reconciliation boundary before any recovery effect",
        ),
        (
            ReservedRecoveryEffectEvidence::PristineNoEffect,
            ReservedRecoveryOwnerObservation::Unavailable,
        ) => (
            "stale_reserved_owner_unproven_preserve_and_hold",
            "owner_process_observation_unavailable",
            "preserve all bytes and restore exact owner-process observation before considering recovery",
        ),
        (ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent, _) => (
            "stale_reserved_effect_state_preserve_and_hold",
            "not_evaluated_effect_evidence_present",
            "preserve all bytes and reconcile the observed effect or ambiguity through the private custody owner",
        ),
    };
    (
        RoutineReservedRecoveryAdmission {
            status,
            owner_observation,
            effect_evidence: match assessment.effect {
                ReservedRecoveryEffectEvidence::PristineNoEffect => "pristine_no_effect_observed",
                ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent => {
                    "effect_or_ambiguity_present"
                }
            },
        },
        next_action,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_assessment_maps_to_a_closed_preservation_hold() {
        let cases = [
            (
                RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::Active,
                    effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
                },
                "stale_reserved_active_owner_preserve_and_retry",
                "exact_owner_process_observed",
                "pristine_no_effect_observed",
            ),
            (
                RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::NotObserved,
                    effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
                },
                "stale_reserved_abandoned_candidate_preserve_and_hold",
                "owner_process_not_observed",
                "pristine_no_effect_observed",
            ),
            (
                RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::Unavailable,
                    effect: ReservedRecoveryEffectEvidence::PristineNoEffect,
                },
                "stale_reserved_owner_unproven_preserve_and_hold",
                "owner_process_observation_unavailable",
                "pristine_no_effect_observed",
            ),
            (
                RoutineReservedRecoveryAssessment {
                    owner: ReservedRecoveryOwnerObservation::Active,
                    effect: ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent,
                },
                "stale_reserved_effect_state_preserve_and_hold",
                "not_evaluated_effect_evidence_present",
                "effect_or_ambiguity_present",
            ),
        ];

        for (assessment, status, owner, effect) in cases {
            let (projected, next_action) = admission(assessment);
            assert_eq!(projected.status, status);
            assert_eq!(projected.owner_observation, owner);
            assert_eq!(projected.effect_evidence, effect);
            assert!(next_action.starts_with("preserve all bytes"));
        }
    }

    #[test]
    fn effect_evidence_takes_precedence_over_owner_observation() {
        for owner in [
            ReservedRecoveryOwnerObservation::Active,
            ReservedRecoveryOwnerObservation::NotObserved,
            ReservedRecoveryOwnerObservation::Unavailable,
        ] {
            let (projected, next_action) = admission(RoutineReservedRecoveryAssessment {
                owner,
                effect: ReservedRecoveryEffectEvidence::EffectOrAmbiguityPresent,
            });
            assert_eq!(
                projected.status,
                "stale_reserved_effect_state_preserve_and_hold"
            );
            assert_eq!(
                projected.owner_observation,
                "not_evaluated_effect_evidence_present"
            );
            assert_eq!(projected.effect_evidence, "effect_or_ambiguity_present");
            assert!(next_action.contains("reconcile the observed effect or ambiguity"));
        }
    }
}
