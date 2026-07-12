use super::frame::{JournalHead, decode_head, decode_log, encode_log, head_for};
use super::lock::JournalLock;
use super::store::Store;
use super::{FileJournal, JournalSnapshot};
use crate::orchestration::{Binding, EventKind, EventLog, OrchestrationError};
use std::path::Path;

impl FileJournal {
    /// Read-only authorization of exactly one expected append beyond a trusted
    /// prior head. This covers a crash after the journal/head commit but before
    /// the caller can retain the returned head.
    pub fn verify_single_advance(
        &self,
        expected_prior: &JournalHead,
        expected_event_id: &str,
        recovered_binding: &Binding,
    ) -> Result<JournalHead, OrchestrationError> {
        let snapshot = self.inspect_unlocked()?;
        let recovered = verify_one_more(
            &snapshot.log,
            expected_prior,
            expected_event_id,
            recovered_binding,
        )?;
        if snapshot.head != recovered {
            return Err(OrchestrationError::JournalCorrupt);
        }
        Ok(recovered)
    }

    /// Opens an existing store without accepting an inconsistent snapshot and
    /// repairs only one authenticated log-publication/head-publication gap.
    pub fn recover_interrupted_append(
        root: impl AsRef<Path>,
        expected_prior: &JournalHead,
        expected_event_id: &str,
        recovered_binding: &Binding,
    ) -> Result<JournalSnapshot, OrchestrationError> {
        let journal = Self {
            store: Store::open(root.as_ref())?,
        };
        journal.store.validate_journal_entries()?;
        journal.repair_one(expected_prior, expected_event_id, recovered_binding)
    }

    fn repair_one(
        &self,
        expected_prior: &JournalHead,
        expected_event_id: &str,
        recovered_binding: &Binding,
    ) -> Result<JournalSnapshot, OrchestrationError> {
        self.store.validate_journal_entries()?;
        let _guard = JournalLock::acquire(&self.store)?;
        let current_head = decode_head(&self.store.read("head.json", 64 * 1024)?)?;
        if &current_head != expected_prior {
            return Err(OrchestrationError::JournalConflict);
        }
        let log_bytes = self.store.read_log()?;
        let log = decode_log(&log_bytes)?;
        let recovered =
            verify_one_more(&log, expected_prior, expected_event_id, recovered_binding)?;
        self.write_head(&recovered)?;
        let snapshot = self.inspect_unlocked()?;
        if snapshot.head != recovered || snapshot.log != log {
            return Err(OrchestrationError::JournalCorrupt);
        }
        self.store.validate_journal_entries()?;
        Ok(snapshot)
    }
}

fn verify_one_more(
    log: &EventLog,
    expected_prior: &JournalHead,
    expected_event_id: &str,
    recovered_binding: &Binding,
) -> Result<JournalHead, OrchestrationError> {
    let prior_count = expected_prior.event_count as usize;
    if log.events().len() != prior_count + 1
        || log.events().last().map(|event| event.event_id.as_str()) != Some(expected_event_id)
    {
        return Err(OrchestrationError::JournalCorrupt);
    }
    let prefix = EventLog(log.events()[..prior_count].to_vec());
    let prefix_bytes = encode_log(&prefix)?;
    if head_for(&expected_prior.binding, &prefix, &prefix_bytes)? != *expected_prior {
        return Err(OrchestrationError::JournalCorrupt);
    }
    let appended = log
        .events()
        .last()
        .ok_or(OrchestrationError::JournalCorrupt)?;
    if appended.binding != expected_prior.binding {
        return Err(OrchestrationError::JournalCorrupt);
    }
    match &appended.event {
        EventKind::CandidateRebound { integration, .. }
            if &integration.integrated_binding == recovered_binding => {}
        EventKind::CandidateRebound { .. } => return Err(OrchestrationError::JournalCorrupt),
        _ if recovered_binding == &expected_prior.binding => {}
        _ => return Err(OrchestrationError::JournalCorrupt),
    }
    let bytes = encode_log(log)?;
    head_for(recovered_binding, log, &bytes)
}
