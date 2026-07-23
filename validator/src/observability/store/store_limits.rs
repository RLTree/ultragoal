use super::*;

impl EventStore {
    pub fn binding_leaf_name(
        context_id: &str,
        candidate_id: &str,
        source_id: &str,
    ) -> Result<String, String> {
        privacy::validate_identifier("context-id", context_id)?;
        privacy::validate_identifier("candidate-id", candidate_id)?;
        privacy::validate_identifier("source-id", source_id)?;
        let mut digest = Sha256::new();
        for value in [context_id, candidate_id, source_id] {
            digest.update((value.len() as u64).to_be_bytes());
            digest.update(value.as_bytes());
        }
        Ok(format!("successor-events-{:x}.jsonl", digest.finalize()))
    }
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
    pub const fn supported_lock_timeout_millis() -> u64 {
        STORE_LOCK_TIMEOUT.as_millis() as u64
    }
    pub fn is_lock_timeout_error(error: &str) -> bool {
        error == LOCK_TIMEOUT_ERROR
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
        let deadline = LockDeadline::for_store_operation()?;
        let (mut file, expected) = self.identity.open_for_append(&deadline)?;
        lock_exclusive(&file, &deadline)?;
        self.identity.validate_bound(&self.path, &file, expected)?;
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
        self.identity.validate_bound(&self.path, &file, expected)?;
        file.write_all(&row).map_err(|_| {
            "observe-append-interrupted: partial tail may require recovery".to_owned()
        })?;
        file.sync_data()
            .map_err(|_| "observe-append-durability-failed".to_owned())?;
        self.identity.validate_bound(&self.path, &file, expected)?;
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
        super::super::explain::explain(self, query, target_event_id)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn validate_query_binding(&self, query: &EventQuery) -> Result<(), String> {
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
}
