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
