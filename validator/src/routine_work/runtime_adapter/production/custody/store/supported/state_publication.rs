use super::*;

#[cfg(test)]
thread_local! {
    static REFUSAL: Cell<Option<usize>> = const { Cell::new(None) };
    static AMBIGUITY: Cell<Option<usize>> = const { Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn set_test_publication_refusal_after(writes: usize) {
    REFUSAL.with(|slot| slot.set(Some(writes)));
}

#[cfg(test)]
pub(crate) fn set_test_publication_ambiguity_after(writes: usize) {
    AMBIGUITY.with(|slot| slot.set(Some(writes)));
}

impl Store {
    pub(super) fn write_initial_state(&self, bytes: &[u8]) -> Result<(), RoutineError> {
        let mut file = self.create_exclusive(STATE_NAME, 0o600)?;
        file.write_all(bytes)
            .map_err(|_| error("routine-production-authority-state-write-failed"))?;
        file.sync_all()
            .map_err(|_| error("routine-production-authority-state-sync-failed"))?;
        self.directory
            .sync_all()
            .map_err(|_| error("routine-production-authority-root-sync-failed"))?;
        let _ = self.exact_identity(STATE_NAME, &file, 0o600)?;
        Ok(())
    }

    pub(super) fn write_atomic_state(&self, bytes: &[u8]) -> StatePublication {
        if bytes.is_empty() || bytes.len() as u64 > MAX_STATE_BYTES || refuse_publication() {
            return StatePublication::Precommit;
        }
        let temporary = match temporary_name() {
            Ok(value) => value,
            Err(_) => return StatePublication::Precommit,
        };
        let mut file = match self.create_exclusive(&temporary, 0o600) {
            Ok(value) => value,
            Err(_) => return StatePublication::Precommit,
        };
        let prepared = (|| {
            file.write_all(bytes)
                .map_err(|_| error("routine-production-authority-state-write-failed"))?;
            file.sync_all()
                .map_err(|_| error("routine-production-authority-state-sync-failed"))?;
            let identity = file_identity(
                &file
                    .metadata()
                    .map_err(|_| error("routine-production-authority-entry-stat-failed"))?,
            );
            if identity.owner != unsafe { libc::geteuid() }
                || identity.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
                || identity.mode & 0o7777 != 0o600
                || identity.links != 1
                || identity.length != bytes.len() as u64
            {
                return Err(error("routine-production-authority-state-identity-invalid"));
            }
            Ok(identity)
        })();
        let Ok(identity) = prepared else {
            return self.precommit_or_ambiguous(&temporary, &file);
        };
        if rename_relative(&self.directory, &temporary, STATE_NAME).is_err() {
            return self.precommit_or_ambiguous(&temporary, &file);
        }
        if self.directory.sync_all().is_err() {
            return StatePublication::Ambiguous;
        }
        match (
            self.exact_identity(STATE_NAME, &file, 0o600),
            self.read_state(),
        ) {
            (Ok(current_identity), Ok(current))
                if current_identity.device == identity.device
                    && current_identity.inode == identity.inode
                    && current == bytes =>
            {
                if obscure_committed_publication() {
                    StatePublication::Ambiguous
                } else {
                    StatePublication::Committed
                }
            }
            _ => StatePublication::Ambiguous,
        }
    }

    fn precommit_or_ambiguous(&self, name: &str, file: &File) -> StatePublication {
        if self.discard_temporary(name, file) {
            StatePublication::Precommit
        } else {
            StatePublication::Ambiguous
        }
    }

    fn discard_temporary(&self, name: &str, file: &File) -> bool {
        let expected = match file.metadata() {
            Ok(metadata) => file_identity(&metadata),
            Err(_) => return false,
        };
        if self.exact_identity(name, file, 0o600).ok() != Some(expected) {
            return false;
        }
        let name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => return false,
        };
        (unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), 0) }) == 0
            && self.directory.sync_all().is_ok()
    }
}

#[cfg(test)]
fn refuse_publication() -> bool {
    countdown(&REFUSAL)
}

#[cfg(test)]
fn obscure_committed_publication() -> bool {
    countdown(&AMBIGUITY)
}

#[cfg(test)]
fn countdown(slot: &'static std::thread::LocalKey<Cell<Option<usize>>>) -> bool {
    slot.with(|slot| match slot.get() {
        Some(0) => {
            slot.set(None);
            true
        }
        Some(remaining) => {
            slot.set(Some(remaining - 1));
            false
        }
        None => false,
    })
}

#[cfg(not(test))]
fn refuse_publication() -> bool {
    false
}

#[cfg(not(test))]
fn obscure_committed_publication() -> bool {
    false
}
