use std::ffi::CString;
use std::os::fd::AsRawFd;

use super::owned_compile_claim::authenticates_claim;
use super::owned_compile_directory::{clear_directory, entry_identity, write_new_file_at_path};
use super::owned_compile_reconciliation::reconcile_displaced_foreign;
use super::owned_compile_scratch::{CleanupState, FAILURE_MARKER, OwnedCompileScratch};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupOutcome {
    RefusedZeroWrite,
    ForeignRestoredNoContentChange,
    Deleted,
    ReconciledForeign,
    AmbiguousPartialEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupStage {
    Authenticated,
    ForeignQuarantined,
    ForeignIdentityValidated,
    ForeignMoved,
    Quarantined,
    Cleared,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupDirective {
    Continue,
    Interrupt,
}

enum QuarantineOutcome {
    Owned(CString),
    Refused,
    ForeignRestored,
    DisplacedForeign {
        quarantine: CString,
        device: u64,
        inode: u64,
    },
}

pub(crate) fn cleanup(scratch: &mut OwnedCompileScratch) -> CleanupOutcome {
    cleanup_controlled(scratch, |_| CleanupDirective::Continue)
}

pub(crate) fn cleanup_controlled(
    scratch: &mut OwnedCompileScratch,
    mut pause: impl FnMut(CleanupStage) -> CleanupDirective,
) -> CleanupOutcome {
    if matches!(
        scratch.cleanup_state,
        CleanupState::DisplacedForeign { .. } | CleanupState::ReconciliationAmbiguous { .. }
    ) {
        return reconcile_displaced_foreign(scratch, &mut pause);
    }
    if matches!(scratch.cleanup_state, CleanupState::Claimed) {
        if !authenticates_claim(scratch) {
            scratch.cleanup_state = CleanupState::Settled;
            return CleanupOutcome::RefusedZeroWrite;
        }
        if pause(CleanupStage::Authenticated) == CleanupDirective::Interrupt {
            scratch.cleanup_state = CleanupState::Settled;
            return CleanupOutcome::RefusedZeroWrite;
        }
        match quarantine_owned_entry(scratch, &mut pause) {
            QuarantineOutcome::Owned(name) => {
                scratch.cleanup_state = CleanupState::Quarantined(name)
            }
            QuarantineOutcome::Refused => {
                scratch.cleanup_state = CleanupState::Settled;
                return CleanupOutcome::RefusedZeroWrite;
            }
            QuarantineOutcome::ForeignRestored => {
                scratch.cleanup_state = CleanupState::Settled;
                return CleanupOutcome::ForeignRestoredNoContentChange;
            }
            QuarantineOutcome::DisplacedForeign {
                quarantine,
                device,
                inode,
            } => {
                scratch.cleanup_state = CleanupState::DisplacedForeign {
                    quarantine,
                    device,
                    inode,
                };
                return CleanupOutcome::AmbiguousPartialEffect;
            }
        }
    }

    let CleanupState::Quarantined(quarantine) = &scratch.cleanup_state else {
        return finish_cleared(scratch, &mut pause);
    };
    if pause(CleanupStage::Quarantined) == CleanupDirective::Interrupt {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if entry_identity(scratch.parent.as_raw_fd(), quarantine)
        != Some((scratch.device, scratch.inode))
        || clear_directory(scratch.directory.as_raw_fd()).is_err()
    {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    scratch.cleanup_state = CleanupState::Cleared(quarantine.clone());
    finish_cleared(scratch, &mut pause)
}

fn finish_cleared(
    scratch: &mut OwnedCompileScratch,
    pause: &mut impl FnMut(CleanupStage) -> CleanupDirective,
) -> CleanupOutcome {
    let CleanupState::Cleared(quarantine) = &scratch.cleanup_state else {
        return CleanupOutcome::RefusedZeroWrite;
    };
    if pause(CleanupStage::Cleared) == CleanupDirective::Interrupt {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    if entry_identity(scratch.parent.as_raw_fd(), quarantine)
        != Some((scratch.device, scratch.inode))
        || unsafe {
            libc::unlinkat(
                scratch.parent.as_raw_fd(),
                quarantine.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } != 0
    {
        return CleanupOutcome::AmbiguousPartialEffect;
    }
    scratch.cleanup_state = CleanupState::Settled;
    if std::thread::panicking() {
        let _ = write_new_file_at_path(
            scratch.parent.as_raw_fd(),
            &scratch.failure_marker,
            FAILURE_MARKER,
        );
    }
    CleanupOutcome::Deleted
}

fn quarantine_owned_entry(
    scratch: &OwnedCompileScratch,
    pause: &mut impl FnMut(CleanupStage) -> CleanupDirective,
) -> QuarantineOutcome {
    let quarantine = random_quarantine_name();
    if unsafe {
        libc::renameatx_np(
            scratch.parent.as_raw_fd(),
            scratch.name.as_ptr(),
            scratch.parent.as_raw_fd(),
            quarantine.as_ptr(),
            libc::RENAME_EXCL,
        )
    } != 0
    {
        return QuarantineOutcome::Refused;
    }
    let Some((device, inode)) = entry_identity(scratch.parent.as_raw_fd(), &quarantine) else {
        return QuarantineOutcome::DisplacedForeign {
            quarantine,
            device: 0,
            inode: 0,
        };
    };
    if (device, inode) == (scratch.device, scratch.inode) {
        return QuarantineOutcome::Owned(quarantine);
    }
    if pause(CleanupStage::ForeignQuarantined) == CleanupDirective::Interrupt {
        return QuarantineOutcome::DisplacedForeign {
            quarantine,
            device,
            inode,
        };
    }
    if unsafe {
        libc::renameatx_np(
            scratch.parent.as_raw_fd(),
            quarantine.as_ptr(),
            scratch.parent.as_raw_fd(),
            scratch.name.as_ptr(),
            libc::RENAME_EXCL,
        )
    } == 0
    {
        QuarantineOutcome::ForeignRestored
    } else {
        QuarantineOutcome::DisplacedForeign {
            quarantine,
            device,
            inode,
        }
    }
}

fn random_quarantine_name() -> CString {
    let mut nonce = [0_u8; 16];
    getrandom::fill(&mut nonce).expect("scratch quarantine randomness unavailable");
    CString::new(format!(".routine-cleanup-{}", hex(&nonce))).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").unwrap();
    }
    value
}
