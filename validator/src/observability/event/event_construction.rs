use super::*;

pub struct SemanticEventInput {
    pub context_id: String,
    pub candidate_id: String,
    pub source_id: String,
    pub event_id: String,
    pub observed_at_unix_ms: u64,
    pub sequence: u64,
    pub operation: String,
    pub outcome: String,
}

impl SemanticEvent {
    pub fn new(input: SemanticEventInput) -> Result<Self, String> {
        let event = Self {
            schema_version: EVENT_SCHEMA.to_owned(),
            event_id: input.event_id,
            context_id: input.context_id,
            candidate_id: input.candidate_id,
            source_id: input.source_id,
            observed_at_unix_ms: input.observed_at_unix_ms,
            sequence: input.sequence,
            parent_event_id: None,
            operation: input.operation,
            outcome: input.outcome,
            duration_ms: None,
            selected_work: None,
            reused_work: None,
            skipped_work: None,
            public_attributes: BTreeMap::new(),
            redacted_attribute_keys: BTreeSet::new(),
            finding_refs: BTreeSet::new(),
            repair_refs: BTreeSet::new(),
        };
        event.validate()?;
        Ok(event)
    }
    pub fn for_context(
        context: &LiveContext,
        source_id: impl Into<String>,
        event_id: impl Into<String>,
        observed_at_unix_ms: u64,
        sequence: u64,
        operation: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<Self, String> {
        Self::new(SemanticEventInput {
            context_id: context.context_id().to_owned(),
            candidate_id: super::super::binding::candidate_id(context)?,
            source_id: source_id.into(),
            event_id: event_id.into(),
            observed_at_unix_ms,
            sequence,
            operation: operation.into(),
            outcome: outcome.into(),
        })
    }
    pub fn from_captured_run(
        run: &CapturedRun,
        source_id: impl Into<String>,
        event_id: impl Into<String>,
        observed_at_unix_ms: u64,
        sequence: u64,
        operation: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<Self, String> {
        let mut event = Self::new(SemanticEventInput {
            context_id: run.observed_context_id().to_owned(),
            candidate_id: run.candidate_id().to_owned(),
            source_id: source_id.into(),
            event_id: event_id.into(),
            observed_at_unix_ms,
            sequence,
            operation: operation.into(),
            outcome: outcome.into(),
        })?;
        event.set_duration_ms(run.monotonic_duration_ns() / 1_000_000)?;
        Ok(event)
    }
    pub fn set_parent(&mut self, event_id: impl Into<String>) -> Result<(), String> {
        let event_id = event_id.into();
        privacy::validate_identifier("parent-event-id", &event_id)?;
        if event_id == self.event_id {
            return Err("observe-invalid-parent: event cannot parent itself".to_owned());
        }
        self.parent_event_id = Some(event_id);
        Ok(())
    }
    pub fn set_duration_ms(&mut self, duration_ms: u64) -> Result<(), String> {
        if duration_ms > MAX_DURATION_MS {
            return Err("observe-duration-limit: duration exceeds the supported bound".to_owned());
        }
        self.duration_ms = Some(duration_ms);
        Ok(())
    }
    pub fn set_work_counts(
        &mut self,
        selected: u64,
        reused: u64,
        skipped: u64,
    ) -> Result<(), String> {
        reused
            .checked_add(skipped)
            .filter(|covered| *covered <= selected)
            .ok_or_else(|| {
                "observe-work-count-invalid: reused plus skipped exceeds selected".to_owned()
            })?;
        self.selected_work = Some(selected);
        self.reused_work = Some(reused);
        self.skipped_work = Some(skipped);
        Ok(())
    }
    pub fn add_public_attribute(&mut self, key: &str, value: &str) -> Result<(), String> {
        if self.public_attributes.len() + self.redacted_attribute_keys.len() >= MAX_ATTRIBUTES
            && !self.public_attributes.contains_key(key)
            && !self.redacted_attribute_keys.contains(key)
        {
            return Err("observe-attribute-limit: attribute cardinality exceeded".to_owned());
        }
        match privacy::sanitize_attribute(key, value)? {
            Some(value) => {
                self.redacted_attribute_keys.remove(key);
                self.public_attributes.insert(key.to_owned(), value);
            }
            None => {
                self.public_attributes.remove(key);
                self.redacted_attribute_keys.insert(key.to_owned());
            }
        }
        self.validate_size()
    }
    pub fn add_sensitive_attribute(&mut self, key: &str, _value: &str) -> Result<(), String> {
        privacy::validate_attribute_key(key)?;
        if privacy::is_claim_authority_key(key) {
            return Err(
                "observe-claim-authority-denied: semantic events cannot carry claim state"
                    .to_owned(),
            );
        }
        if self.public_attributes.len() + self.redacted_attribute_keys.len() >= MAX_ATTRIBUTES
            && !self.public_attributes.contains_key(key)
            && !self.redacted_attribute_keys.contains(key)
        {
            return Err("observe-attribute-limit: attribute cardinality exceeded".to_owned());
        }
        self.public_attributes.remove(key);
        self.redacted_attribute_keys.insert(key.to_owned());
        self.validate_size()
    }
    pub fn add_finding(&mut self, finding: &Finding) -> Result<(), String> {
        self.add_finding_ref(&finding.finding_id)?;
        self.add_repair_ref(&finding.repair.repair_id)
    }
    pub fn add_finding_ref(&mut self, finding_id: &str) -> Result<(), String> {
        super::super::binding::add_reference(&mut self.finding_refs, finding_id, "finding")
    }
    pub fn add_repair_ref(&mut self, repair_id: &str) -> Result<(), String> {
        super::super::binding::add_reference(&mut self.repair_refs, repair_id, "repair")
    }
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
    pub fn observed_at_unix_ms(&self) -> u64 {
        self.observed_at_unix_ms
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn parent_event_id(&self) -> Option<&str> {
        self.parent_event_id.as_deref()
    }
    pub fn operation(&self) -> &str {
        &self.operation
    }
    pub fn outcome(&self) -> &str {
        &self.outcome
    }
}
