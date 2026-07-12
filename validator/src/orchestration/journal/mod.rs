mod frame;
mod lock;
mod recovery;
mod store;
mod sys;
#[cfg(test)]
mod test_hook;

pub use frame::{JournalHead, JournalSnapshot};

use self::frame::{decode_head, decode_log, encode_log, head_for, validate_snapshot};
use self::lock::JournalLock;
use self::store::Store;
use super::{Binding, EventLog, OrchestrationError};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct FileJournal {
    store: Store,
}

impl FileJournal {
    pub(crate) fn create(
        root: impl AsRef<Path>,
        binding: &Binding,
        log: &EventLog,
    ) -> Result<(Self, JournalHead), OrchestrationError> {
        let store = Store::create(root.as_ref())?;
        let journal = Self { store };
        let _guard = JournalLock::acquire(&journal.store)?;
        for name in ["events.jsonl", "head.json"] {
            journal.store.require_absent(name)?;
        }
        let bytes = encode_log(log)?;
        let head = head_for(binding, log, &bytes)?;
        journal.store.write_atomic("events.jsonl", &bytes)?;
        journal.write_head(&head)?;
        Ok((journal, head))
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self, OrchestrationError> {
        let journal = Self {
            store: Store::open(root.as_ref())?,
        };
        journal.store.validate_journal_entries()?;
        journal.inspect()?;
        Ok(journal)
    }

    /// Read-only inspection. It does not acquire or create the journal lock.
    pub fn inspect(&self) -> Result<JournalSnapshot, OrchestrationError> {
        self.inspect_unlocked()
    }

    pub fn path(&self) -> PathBuf {
        self.store.root().to_path_buf()
    }

    pub(crate) fn append(
        &self,
        expected: &JournalHead,
        binding: &Binding,
        log: &EventLog,
    ) -> Result<JournalHead, OrchestrationError> {
        let _guard = JournalLock::acquire(&self.store)?;
        let (bytes, next) = self.prepare_append(expected, binding, log)?;
        self.store.write_atomic("events.jsonl", &bytes)?;
        self.write_head(&next)?;
        Ok(next)
    }

    #[cfg(test)]
    pub(crate) fn crash_after_log_publication(
        &self,
        expected: &JournalHead,
        binding: &Binding,
        log: &EventLog,
    ) -> ! {
        let result = (|| {
            let _guard = JournalLock::acquire(&self.store)?;
            let (bytes, _) = self.prepare_append(expected, binding, log)?;
            self.store.write_atomic("events.jsonl", &bytes)
        })();
        match result {
            Ok(()) => std::process::exit(87),
            Err(error) => panic!("failed to establish interrupted publication: {error}"),
        }
    }

    fn prepare_append(
        &self,
        expected: &JournalHead,
        binding: &Binding,
        log: &EventLog,
    ) -> Result<(Vec<u8>, JournalHead), OrchestrationError> {
        let current = self.inspect_unlocked()?;
        if &current.head != expected
            || log.events().len() != current.log.events().len() + 1
            || log.events()[..current.log.events().len()] != *current.log.events()
        {
            return Err(OrchestrationError::JournalConflict);
        }
        let bytes = encode_log(log)?;
        let next = head_for(binding, log, &bytes)?;
        Ok((bytes, next))
    }

    fn inspect_unlocked(&self) -> Result<JournalSnapshot, OrchestrationError> {
        self.store.validate_journal_entries()?;
        let log_bytes = self.store.read_log()?;
        let head_bytes = self.store.read("head.json", 64 * 1024)?;
        let head = decode_head(&head_bytes)?;
        let log = decode_log(&log_bytes)?;
        validate_snapshot(&head, &log, &log_bytes)?;
        self.store.validate_journal_entries()?;
        Ok(JournalSnapshot { head, log })
    }

    #[cfg(test)]
    pub(crate) fn inspect_with_log_name_hook(
        &self,
        hook: impl FnOnce(),
    ) -> Result<JournalSnapshot, OrchestrationError> {
        self.store.validate_journal_entries()?;
        let log_bytes = self.store.read_log_with_hook(hook)?;
        let head = decode_head(&self.store.read("head.json", 64 * 1024)?)?;
        let log = decode_log(&log_bytes)?;
        validate_snapshot(&head, &log, &log_bytes)?;
        self.store.validate_journal_entries()?;
        Ok(JournalSnapshot { head, log })
    }

    fn write_head(&self, head: &JournalHead) -> Result<(), OrchestrationError> {
        let mut bytes = serde_json::to_vec(head).map_err(|_| OrchestrationError::JournalCorrupt)?;
        bytes.push(b'\n');
        self.store.write_atomic("head.json", &bytes)
    }

    #[cfg(test)]
    pub(crate) fn set_test_pre_publication_hook(hook: impl FnOnce() + 'static) {
        test_hook::set(hook);
    }
}
