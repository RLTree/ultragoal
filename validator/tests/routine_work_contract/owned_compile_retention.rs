use super::owned_compile_claim::authenticates_claim;
use super::owned_compile_scratch::{CleanupState, OwnedCompileScratch};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupOutcome {
    RefusedZeroWrite,
    RetainedNoDestructiveAuthority,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupStage {
    RootQuarantineRenameRetired,
    ForeignRecoveryRenameRetired,
    RootDirectoryUnlinkRetired,
    DescendantDirectoryUnlinkRetired,
    DescendantEntryUnlinkRetired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupDirective {
    Continue,
    Interrupt,
}

const RETIRED_TRANSITIONS: [CleanupStage; 5] = [
    CleanupStage::RootQuarantineRenameRetired,
    CleanupStage::ForeignRecoveryRenameRetired,
    CleanupStage::RootDirectoryUnlinkRetired,
    CleanupStage::DescendantDirectoryUnlinkRetired,
    CleanupStage::DescendantEntryUnlinkRetired,
];

pub(crate) fn retain(scratch: &mut OwnedCompileScratch) -> CleanupOutcome {
    retain_controlled(scratch, |_| CleanupDirective::Continue)
}

pub(crate) fn retain_controlled(
    scratch: &mut OwnedCompileScratch,
    mut pause: impl FnMut(CleanupStage) -> CleanupDirective,
) -> CleanupOutcome {
    if !authenticates_claim(scratch) {
        scratch.cleanup_state = CleanupState::Refused;
        return CleanupOutcome::RefusedZeroWrite;
    }
    for transition in RETIRED_TRANSITIONS {
        if pause(transition) == CleanupDirective::Interrupt || !authenticates_claim(scratch) {
            scratch.cleanup_state = CleanupState::Refused;
            return CleanupOutcome::RefusedZeroWrite;
        }
    }
    scratch.cleanup_state = CleanupState::Retained;
    CleanupOutcome::RetainedNoDestructiveAuthority
}
