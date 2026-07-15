use std::os::fd::AsRawFd;

use super::owned_compile_directory::entry_identity;
use super::owned_compile_quarantine::{CleanupDirective, CleanupOutcome, CleanupStage};
use super::owned_compile_scratch::{CleanupState, OwnedCompileScratch};

pub(crate) fn reconcile_displaced_foreign(
    scratch: &mut OwnedCompileScratch,
    pause: &mut impl FnMut(CleanupStage) -> CleanupDirective,
) -> CleanupOutcome {
    let (quarantine, device, inode) = match &scratch.cleanup_state {
        CleanupState::DisplacedForeign {
            quarantine,
            device,
            inode,
        }
        | CleanupState::ReconciliationAmbiguous {
            quarantine,
            device,
            inode,
            ..
        } => (quarantine.clone(), *device, *inode),
        _ => return CleanupOutcome::AmbiguousPartialEffect,
    };
    if entry_identity(scratch.parent.as_raw_fd(), &scratch.name) == Some((device, inode)) {
        if entry_identity(scratch.parent.as_raw_fd(), &quarantine).is_none() {
            scratch.cleanup_state = CleanupState::Settled;
            return CleanupOutcome::ReconciledForeign;
        }
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if entry_identity(scratch.parent.as_raw_fd(), &quarantine) != Some((device, inode))
        || entry_identity(scratch.parent.as_raw_fd(), &scratch.name).is_some()
    {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if pause(CleanupStage::ForeignIdentityValidated) == CleanupDirective::Interrupt {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if unsafe {
        libc::renameatx_np(
            scratch.parent.as_raw_fd(),
            quarantine.as_ptr(),
            scratch.parent.as_raw_fd(),
            scratch.name.as_ptr(),
            libc::RENAME_EXCL,
        )
    } != 0
    {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if pause(CleanupStage::ForeignMoved) == CleanupDirective::Interrupt {
        retain_ambiguous_destination(scratch, quarantine, device, inode);
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    let destination = entry_identity(scratch.parent.as_raw_fd(), &scratch.name);
    if destination == Some((device, inode))
        && entry_identity(scratch.parent.as_raw_fd(), &quarantine).is_none()
    {
        scratch.cleanup_state = CleanupState::Settled;
        CleanupOutcome::ReconciledForeign
    } else {
        scratch.cleanup_state = CleanupState::ReconciliationAmbiguous {
            quarantine,
            device,
            inode,
            destination,
        };
        CleanupOutcome::AmbiguousPartialEffect
    }
}

fn retain_ambiguous_destination(
    scratch: &mut OwnedCompileScratch,
    quarantine: std::ffi::CString,
    device: u64,
    inode: u64,
) {
    scratch.cleanup_state = CleanupState::ReconciliationAmbiguous {
        quarantine,
        device,
        inode,
        destination: entry_identity(scratch.parent.as_raw_fd(), &scratch.name),
    };
}
