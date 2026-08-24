use super::*;

impl EventStore {
    pub(crate) fn read_events(&self) -> Result<Vec<SemanticEvent>, String> {
        let deadline = LockDeadline::for_store_operation()?;
        let initial_expected = self.identity.expected(&deadline)?;
        let Some(mut file) = filesystem::open_read(self.identity.parent(), initial_expected)?
        else {
            return Ok(Vec::new());
        };
        lock_shared(&file, &deadline)?;
        let expected = match initial_expected {
            Some(expected) => expected,
            None => self.identity.expected(&deadline)?.ok_or_else(|| {
                "observe-store-path-denied: store materialized outside append".to_owned()
            })?,
        };
        self.identity.validate_bound(&self.path, &file, expected)?;
        let bytes = filesystem::read_bounded(&mut file, self.max_store_bytes)?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        let decoded = format::decode(&bytes, self.max_scan_rows)?;
        self.validate_decoded(&decoded)?;
        Ok(decoded.events)
    }
    pub(crate) fn validate_event_binding(&self, event: &SemanticEvent) -> Result<(), String> {
        if event.context_id() != self.context_id {
            return Err("observe-binding-stale-or-wrong-context".to_owned());
        }
        if event.candidate_id() != self.candidate_id {
            return Err("observe-binding-stale-or-wrong-candidate".to_owned());
        }
        if event.source_id() != self.source_id {
            return Err("observe-binding-stale-or-wrong-source".to_owned());
        }
        Ok(())
    }
    pub(crate) fn read_events_for_migration_admission(
        &self,
    ) -> Result<(Vec<SemanticEvent>, bool), String> {
        let deadline = LockDeadline::for_store_operation()?;
        let initial_expected = self.identity.expected(&deadline)?;
        let Some(mut file) = filesystem::open_read(self.identity.parent(), initial_expected)?
        else {
            return Ok((Vec::new(), false));
        };
        lock_shared(&file, &deadline)?;
        let expected = match initial_expected {
            Some(expected) => expected,
            None => self.identity.expected(&deadline)?.ok_or_else(|| {
                "observe-store-path-denied: store materialized outside append".to_owned()
            })?,
        };
        self.identity.validate_bound(&self.path, &file, expected)?;
        let bytes = filesystem::read_bounded(&mut file, self.max_store_bytes)?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        let decoded = format::decode_conflict_tolerant(&bytes, self.max_scan_rows)?;
        if decoded.events.len() > self.max_events || decoded.physical_row_count > self.max_events {
            return Err("observe-event-limit: store event cardinality exceeded".to_owned());
        }
        self.validate_rows(&decoded.events)?;
        Ok((decoded.events, decoded.duplicate_or_conflicting_event_id))
    }
    pub(in crate::observability) fn validate_decoded(
        &self,
        decoded: &format::DecodedRows,
    ) -> Result<(), String> {
        if decoded.events.len() > self.max_events || decoded.physical_row_count > self.max_events {
            return Err("observe-event-limit: store event cardinality exceeded".to_owned());
        }
        self.validate_rows(&decoded.events)
    }
    pub(crate) fn validate_rows(&self, events: &[SemanticEvent]) -> Result<(), String> {
        for event in events {
            self.validate_event_binding(event)
                .map_err(|_| "observe-store-corrupt:row-binding-conflict".to_owned())?;
        }
        Ok(())
    }
}

pub(crate) fn stable_sort(events: &mut [SemanticEvent]) {
    events.sort_by(|left, right| {
        (left.observed_at_unix_ms(), left.sequence(), left.event_id()).cmp(&(
            right.observed_at_unix_ms(),
            right.sequence(),
            right.event_id(),
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::SemanticEventInput;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn fixture(label: &str, bytes: &[u8]) -> (std::path::PathBuf, EventStore) {
        let root = fs::canonicalize(std::env::temp_dir())
            .unwrap()
            .join(format!(
                "hul-observe-tolerant-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&root).unwrap();
        let path = root.join("events.jsonl");
        fs::write(&path, bytes).unwrap();
        let store = EventStore::open_bound(&path, "context", "candidate", "source").unwrap();
        (root, store)
    }

    fn event(id: &str, context: &str) -> SemanticEvent {
        SemanticEvent::new(SemanticEventInput {
            context_id: context.to_owned(),
            candidate_id: "candidate".to_owned(),
            source_id: "source".to_owned(),
            event_id: id.to_owned(),
            observed_at_unix_ms: 1,
            sequence: 1,
            operation: "check.test".to_owned(),
            outcome: "complete".to_owned(),
        })
        .unwrap()
    }

    #[test]
    fn migration_reader_is_tolerant_only_after_complete_descriptor_bound_validation() {
        let row = format::encode(&event("duplicate", "context")).unwrap();
        let (root, store) = fixture("duplicate", &[row.as_slice(), row.as_slice()].concat());
        assert_eq!(
            store.read_events().err().unwrap(),
            "observe-store-corrupt:row-2-duplicate-event"
        );
        let (events, conflict) = store.read_events_for_migration_admission().unwrap();
        assert_eq!(events.len(), 2);
        assert!(conflict);
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_reader_rejects_any_valid_row_with_the_wrong_store_binding() {
        let correct = format::encode(&event("correct", "context")).unwrap();
        let wrong = format::encode(&event("wrong", "other-context")).unwrap();
        let (root, store) = fixture(
            "binding",
            &[correct.as_slice(), correct.as_slice(), wrong.as_slice()].concat(),
        );
        assert_eq!(
            store.read_events_for_migration_admission().err().unwrap(),
            "observe-store-corrupt:row-binding-conflict"
        );
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn migration_reader_returns_the_full_validated_scan_without_query_truncation() {
        let rows = (0..=EventStore::supported_result_limit())
            .map(|index| format::encode(&event(&format!("event-{index}"), "context")).unwrap())
            .collect::<Vec<_>>();
        let bytes = rows
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect::<Vec<_>>();
        let (root, store) = fixture("full-scan", &bytes);
        let (events, conflict) = store.read_events_for_migration_admission().unwrap();
        assert_eq!(events.len(), EventStore::supported_result_limit() + 1);
        assert!(!conflict);
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }
}
