use super::filesystem;
use super::format;
use super::identity::BoundStoreIdentity;
use super::limits::{HARD_MAX_EVENTS, HARD_MAX_RESULTS, HARD_MAX_SCAN_ROWS, HARD_MAX_STORE_BYTES};
use super::privacy;
use super::{CausalExplanation, EventQuery, SemanticEvent};
use crate::context::LiveContext;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct EventStore {
    pub(super) path: PathBuf,
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) source_id: String,
    pub(super) max_store_bytes: u64,
    pub(super) max_events: usize,
    pub(super) max_scan_rows: usize,
    pub(super) max_results: usize,
    pub(super) identity: BoundStoreIdentity,
}

impl EventStore {
    pub const fn supported_store_limit_bytes() -> u64 {
        HARD_MAX_STORE_BYTES
    }

    pub const fn supported_event_limit() -> usize {
        HARD_MAX_EVENTS
    }

    pub const fn supported_scan_limit() -> usize {
        HARD_MAX_SCAN_ROWS
    }

    pub const fn supported_result_limit() -> usize {
        HARD_MAX_RESULTS
    }

    pub fn open_bound(
        path: impl Into<PathBuf>,
        context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        source_id: impl Into<String>,
    ) -> Result<Self, String> {
        let path = path.into();
        let context_id = context_id.into();
        let candidate_id = candidate_id.into();
        let source_id = source_id.into();
        privacy::validate_identifier("context-id", &context_id)?;
        privacy::validate_identifier("candidate-id", &candidate_id)?;
        privacy::validate_identifier("source-id", &source_id)?;
        let parent = filesystem::bind_parent(&path)?;
        let initial_identity = filesystem::identity_if_exists(&parent)?;
        Ok(Self {
            path,
            context_id,
            candidate_id,
            source_id,
            max_store_bytes: HARD_MAX_STORE_BYTES,
            max_events: HARD_MAX_EVENTS,
            max_scan_rows: HARD_MAX_SCAN_ROWS,
            max_results: HARD_MAX_RESULTS,
            identity: BoundStoreIdentity::new(initial_identity, parent),
        })
    }

    pub fn for_context(
        path: impl Into<PathBuf>,
        context: &LiveContext,
        source_id: impl Into<String>,
    ) -> Result<Self, String> {
        let binding = SemanticEvent::for_context(
            context,
            source_id,
            "store-binding",
            0,
            0,
            "observe.store",
            "unknown",
        )?;
        Self::open_bound(
            path,
            binding.context_id(),
            binding.candidate_id(),
            binding.source_id(),
        )
    }

    pub fn with_store_limit(mut self, bytes: u64) -> Result<Self, String> {
        if !(1..=HARD_MAX_STORE_BYTES).contains(&bytes) {
            return Err("observe-store-limit: configured byte bound is unsupported".to_owned());
        }
        self.max_store_bytes = bytes;
        Ok(self)
    }

    pub fn with_event_limit(mut self, events: usize) -> Result<Self, String> {
        if !(1..=HARD_MAX_EVENTS).contains(&events) {
            return Err("observe-event-limit: configured event bound is unsupported".to_owned());
        }
        self.max_events = events;
        Ok(self)
    }

    pub fn with_scan_limit(mut self, rows: usize) -> Result<Self, String> {
        if !(1..=HARD_MAX_SCAN_ROWS).contains(&rows) {
            return Err("observe-scan-limit: configured row bound is unsupported".to_owned());
        }
        self.max_scan_rows = rows;
        Ok(self)
    }

    pub fn with_result_limit(mut self, results: usize) -> Result<Self, String> {
        if !(1..=HARD_MAX_RESULTS).contains(&results) {
            return Err("observe-query-limit: configured result bound is unsupported".to_owned());
        }
        self.max_results = results;
        Ok(self)
    }

    /// Append one row. `Ok(false)` means the exact event was already present.
    pub fn append(&self, event: &SemanticEvent) -> Result<bool, String> {
        event.validate()?;
        self.validate_event_binding(event)?;
        let row = format::encode(event)?;
        let mut file = self.identity.open_for_append()?;
        lock_exclusive(&file)?;
        self.identity.validate_bound(&self.path, &file)?;
        let bytes = filesystem::read_bounded(&mut file, self.max_store_bytes)?;
        let decoded = format::decode(&bytes, self.max_scan_rows)?;
        self.validate_rows(&decoded.events)?;
        if let Some(existing) = decoded
            .events
            .iter()
            .find(|item| item.event_id() == event.event_id())
        {
            return if existing == event {
                Ok(false)
            } else {
                Err("observe-duplicate-conflict: event id already has different content".to_owned())
            };
        }
        if decoded.events.len() >= self.max_events || decoded.physical_row_count >= self.max_events
        {
            return Err("observe-event-limit: store event cardinality exceeded".to_owned());
        }
        let next_size = bytes
            .len()
            .checked_add(row.len())
            .ok_or_else(|| "observe-store-limit: size overflow".to_owned())?;
        if next_size as u64 > self.max_store_bytes {
            return Err("observe-store-limit: append would exceed byte bound".to_owned());
        }
        self.identity.validate_bound(&self.path, &file)?;
        file.write_all(&row).map_err(|_| {
            "observe-append-interrupted: partial tail may require recovery".to_owned()
        })?;
        file.sync_data()
            .map_err(|_| "observe-append-durability-failed".to_owned())?;
        self.identity.validate_bound(&self.path, &file)?;
        Ok(true)
    }

    pub fn query(&self, query: &EventQuery) -> Result<Vec<SemanticEvent>, String> {
        self.validate_query_binding(query)?;
        let mut events = self.read_events()?;
        events.retain(|event| query.matches(event));
        stable_sort(&mut events);
        let limit = query.limit.min(self.max_results);
        if events.len() > limit {
            events.truncate(limit);
        }
        Ok(events)
    }

    pub fn explain(
        &self,
        query: &EventQuery,
        target_event_id: &str,
    ) -> Result<CausalExplanation, String> {
        super::explain::explain(self, query, target_event_id)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn validate_query_binding(&self, query: &EventQuery) -> Result<(), String> {
        query.validate()?;
        if query.context_id != self.context_id {
            return Err("observe-binding-wrong-context".to_owned());
        }
        if query.candidate_id != self.candidate_id {
            return Err("observe-binding-stale-candidate".to_owned());
        }
        if query.source_id != self.source_id {
            return Err("observe-binding-wrong-source".to_owned());
        }
        if query.limit > self.max_results {
            return Err("observe-query-limit: requested results exceed store policy".to_owned());
        }
        Ok(())
    }

    pub(super) fn read_events(&self) -> Result<Vec<SemanticEvent>, String> {
        let Some(mut file) =
            filesystem::open_read(self.identity.parent(), self.identity.expected()?)?
        else {
            return Ok(Vec::new());
        };
        lock_shared(&file)?;
        self.identity.validate_bound(&self.path, &file)?;
        let bytes = filesystem::read_bounded(&mut file, self.max_store_bytes)?;
        self.identity.validate_bound(&self.path, &file)?;
        let decoded = format::decode(&bytes, self.max_scan_rows)?;
        self.validate_decoded(&decoded)?;
        Ok(decoded.events)
    }

    fn validate_event_binding(&self, event: &SemanticEvent) -> Result<(), String> {
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

    pub(super) fn validate_decoded(&self, decoded: &format::DecodedRows) -> Result<(), String> {
        if decoded.events.len() > self.max_events || decoded.physical_row_count > self.max_events {
            return Err("observe-event-limit: store event cardinality exceeded".to_owned());
        }
        self.validate_rows(&decoded.events)
    }

    pub(super) fn validate_rows(&self, events: &[SemanticEvent]) -> Result<(), String> {
        for event in events {
            self.validate_event_binding(event)
                .map_err(|_| "observe-store-corrupt:row-binding-conflict".to_owned())?;
        }
        Ok(())
    }
}

fn stable_sort(events: &mut [SemanticEvent]) {
    events.sort_by(|left, right| {
        (left.observed_at_unix_ms(), left.sequence(), left.event_id()).cmp(&(
            right.observed_at_unix_ms(),
            right.sequence(),
            right.event_id(),
        ))
    });
}

fn lock_shared(file: &File) -> Result<(), String> {
    file.lock_shared()
        .map_err(|_| "observe-store-lock-failed".to_owned())
}

fn lock_exclusive(file: &File) -> Result<(), String> {
    file.lock()
        .map_err(|_| "observe-store-lock-failed".to_owned())
}
