use std::os::fd::AsRawFd;

use super::owned_compile_directory::entry_identity;
use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::{CleanupState, OwnedCompileScratch};

pub(crate) fn reconcile_displaced_foreign(
    scratch: &mut OwnedCompileScratch,
    pause: &mut impl FnMut(CleanupStage) -> CleanupDirective,
) -> CleanupOutcome {
    let CleanupState::DisplacedForeign {
        quarantine,
        custody,
        destination,
    } = &mut scratch.cleanup_state
    else {
        return CleanupOutcome::AmbiguousPartialEffect;
    };
    let parent_path = scratch.path.parent().unwrap();
    let Some(source_name) = custody.current_name(parent_path) else {
        return CleanupOutcome::AmbiguousPartialEffect;
    };
    if custody.matches(&scratch.parent, &scratch.name) {
        if entry_identity(scratch.parent.as_raw_fd(), quarantine).is_none() {
            scratch.cleanup_state = CleanupState::Settled;
            return CleanupOutcome::ReconciledForeign;
        }
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    *destination = entry_identity(scratch.parent.as_raw_fd(), &scratch.name);
    if !custody.matches(&scratch.parent, &source_name) || destination.is_some() {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if pause(CleanupStage::ForeignIdentityValidated) == CleanupDirective::Interrupt {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if unsafe {
        libc::renameatx_np(
            scratch.parent.as_raw_fd(),
            source_name.as_ptr(),
            scratch.parent.as_raw_fd(),
            scratch.name.as_ptr(),
            libc::RENAME_EXCL,
        )
    } != 0
    {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    let interrupted = pause(CleanupStage::ForeignMoved) == CleanupDirective::Interrupt;
    *destination = entry_identity(scratch.parent.as_raw_fd(), &scratch.name);
    let moved_exact = custody.matches(&scratch.parent, &scratch.name);
    let source_absent = entry_identity(scratch.parent.as_raw_fd(), &source_name).is_none();
    let quarantine_absent = entry_identity(scratch.parent.as_raw_fd(), quarantine).is_none();
    if !interrupted && moved_exact && source_absent && quarantine_absent {
        scratch.cleanup_state = CleanupState::Settled;
        CleanupOutcome::ReconciledForeign
    } else {
        CleanupOutcome::AmbiguousPartialEffect
    }
}
