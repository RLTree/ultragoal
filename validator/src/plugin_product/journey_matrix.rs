use super::lifecycle::{LifecycleEffect, LifecycleIntent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JourneyTruthCeiling {
    SourceCandidateOnly,
    LiveHostProofRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JourneyDefinition {
    pub id: &'static str,
    pub intent: LifecycleIntent,
    pub desired_outcome: &'static str,
    pub failure_outcome: &'static str,
    pub required_effects: &'static [LifecycleEffect],
    pub truth_ceiling: JourneyTruthCeiling,
}

const INSTALL_EFFECTS: &[LifecycleEffect] = &[
    LifecycleEffect::InstallPackage,
    LifecycleEffect::RefreshCache,
    LifecycleEffect::ProbeRuntime,
];
const UPDATE_EFFECTS: &[LifecycleEffect] = &[
    LifecycleEffect::InstallPackage,
    LifecycleEffect::RefreshCache,
    LifecycleEffect::ProbeRuntime,
];
const RECOVERY_EFFECTS: &[LifecycleEffect] = &[
    LifecycleEffect::RestorePriorAuthority,
    LifecycleEffect::ProbeRuntime,
];
const REINSTALL_EFFECTS: &[LifecycleEffect] = &[
    LifecycleEffect::VerifyInstalledBytes,
    LifecycleEffect::ProbeRuntime,
];
const UNINSTALL_EFFECTS: &[LifecycleEffect] = &[
    LifecycleEffect::RemoveInstalledPackage,
    LifecycleEffect::RemoveCache,
    LifecycleEffect::VerifyTeardown,
];
const CACHE_EFFECTS: &[LifecycleEffect] =
    &[LifecycleEffect::RefreshCache, LifecycleEffect::ProbeRuntime];
const REPEAT_EFFECTS: &[LifecycleEffect] = &[
    LifecycleEffect::VerifyInstalledBytes,
    LifecycleEffect::ProbeRuntime,
];

pub const JOURNEYS: [JourneyDefinition; 8] = [
    JourneyDefinition {
        id: "fresh-install",
        intent: LifecycleIntent::FreshInstall,
        desired_outcome: "authorized package becomes installed, cache-bound, and runtime-probed",
        failure_outcome: "absence is preserved and the failed effect is identified",
        required_effects: INSTALL_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "monotonic-update",
        intent: LifecycleIntent::MonotonicUpdate,
        desired_outcome: "strictly newer package replaces the expected prior authority",
        failure_outcome: "the prior installed and cache authority is restored",
        required_effects: UPDATE_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "failed-update-recovery",
        intent: LifecycleIntent::FailedUpdateRecovery,
        desired_outcome: "the captured prior authority is restored and runtime-probed",
        failure_outcome: "recovery remains required and no higher claim is raised",
        required_effects: RECOVERY_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "authorized-rollback",
        intent: LifecycleIntent::AuthorizedRollback,
        desired_outcome: "an explicitly authorized older package becomes current",
        failure_outcome: "the newer prior authority is restored",
        required_effects: UPDATE_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "idempotent-reinstall",
        intent: LifecycleIntent::IdempotentReinstall,
        desired_outcome: "matching installed bytes are verified without replacement",
        failure_outcome: "the mismatch is reported without rewriting host state",
        required_effects: REINSTALL_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "uninstall-teardown",
        intent: LifecycleIntent::UninstallTeardown,
        desired_outcome: "installed and cache authority are removed and absence is verified",
        failure_outcome: "the prior authority is restored or recovery is required",
        required_effects: UNINSTALL_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "stale-cache-recovery",
        intent: LifecycleIntent::StaleCacheRecovery,
        desired_outcome: "cache identity is reconciled to installed bytes",
        failure_outcome: "installed authority remains unchanged and stale cache is named",
        required_effects: CACHE_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
    JourneyDefinition {
        id: "repeat-use",
        intent: LifecycleIntent::RepeatUse,
        desired_outcome: "matching installed bytes are reused and runtime behavior is re-observed",
        failure_outcome: "reuse is refused when candidate identity drifts",
        required_effects: REPEAT_EFFECTS,
        truth_ceiling: JourneyTruthCeiling::LiveHostProofRequired,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JourneyMatrixError {
    DuplicateId,
    DuplicateIntent,
    EmptyOutcome,
    EmptyEffects,
    WrongCardinality,
}

pub fn validate_journey_matrix() -> Result<(), JourneyMatrixError> {
    if JOURNEYS.len() != 8 {
        return Err(JourneyMatrixError::WrongCardinality);
    }
    let mut ids = BTreeSet::new();
    let mut intents = BTreeSet::new();
    for journey in &JOURNEYS {
        if !ids.insert(journey.id) {
            return Err(JourneyMatrixError::DuplicateId);
        }
        if !intents.insert(journey.intent) {
            return Err(JourneyMatrixError::DuplicateIntent);
        }
        if journey.desired_outcome.is_empty() || journey.failure_outcome.is_empty() {
            return Err(JourneyMatrixError::EmptyOutcome);
        }
        if journey.required_effects.is_empty() {
            return Err(JourneyMatrixError::EmptyEffects);
        }
    }
    Ok(())
}
