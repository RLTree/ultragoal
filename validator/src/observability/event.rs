use super::limits::{EVENT_SCHEMA, MAX_ATTRIBUTES, MAX_DURATION_MS, MAX_REFERENCES, MAX_ROW_BYTES};
use super::privacy;
use crate::capture::CapturedRun;
use crate::context::LiveContext;
use crate::state::Finding;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A bounded observation. It deliberately has no claim-state field or claim mutation API.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticEvent {
    schema_version: String,
    event_id: String,
    context_id: String,
    candidate_id: String,
    source_id: String,
    observed_at_unix_ms: u64,
    sequence: u64,
    parent_event_id: Option<String>,
    operation: String,
    outcome: String,
    duration_ms: Option<u64>,
    selected_work: Option<u64>,
    reused_work: Option<u64>,
    skipped_work: Option<u64>,
    public_attributes: BTreeMap<String, String>,
    redacted_attribute_keys: BTreeSet<String>,
    finding_refs: BTreeSet<String>,
    repair_refs: BTreeSet<String>,
}

impl SemanticEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        source_id: impl Into<String>,
        event_id: impl Into<String>,
        observed_at_unix_ms: u64,
        sequence: u64,
        operation: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<Self, String> {
        let event = Self {
            schema_version: EVENT_SCHEMA.to_owned(),
            event_id: event_id.into(),
            context_id: context_id.into(),
            candidate_id: candidate_id.into(),
            source_id: source_id.into(),
            observed_at_unix_ms,
            sequence,
            parent_event_id: None,
            operation: operation.into(),
            outcome: outcome.into(),
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
        Self::new(
            context.context_id(),
            super::binding::candidate_id(context)?,
            source_id,
            event_id,
            observed_at_unix_ms,
            sequence,
            operation,
            outcome,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_captured_run(
        run: &CapturedRun,
        source_id: impl Into<String>,
        event_id: impl Into<String>,
        observed_at_unix_ms: u64,
        sequence: u64,
        operation: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Result<Self, String> {
        let mut event = Self::new(
            run.observed_context_id(),
            run.candidate_id(),
            source_id,
            event_id,
            observed_at_unix_ms,
            sequence,
            operation,
            outcome,
        )?;
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
        super::binding::add_reference(&mut self.finding_refs, finding_id, "finding")
    }

    pub fn add_repair_ref(&mut self, repair_id: &str) -> Result<(), String> {
        super::binding::add_reference(&mut self.repair_refs, repair_id, "repair")
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
    pub fn redacted_attribute_keys(&self) -> &BTreeSet<String> {
        &self.redacted_attribute_keys
    }
    pub fn finding_refs(&self) -> &BTreeSet<String> {
        &self.finding_refs
    }
    pub fn repair_refs(&self) -> &BTreeSet<String> {
        &self.repair_refs
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema_version != EVENT_SCHEMA {
            return Err("observe-store-corrupt:unsupported-event-version".to_owned());
        }
        for (label, value) in [
            ("event-id", self.event_id.as_str()),
            ("context-id", self.context_id.as_str()),
            ("candidate-id", self.candidate_id.as_str()),
            ("source-id", self.source_id.as_str()),
            ("operation", self.operation.as_str()),
            ("outcome", self.outcome.as_str()),
        ] {
            privacy::validate_identifier(label, value)?;
        }
        if self.parent_event_id.as_deref() == Some(self.event_id.as_str()) {
            return Err("observe-store-corrupt:self-parent".to_owned());
        }
        if self.public_attributes.len() + self.redacted_attribute_keys.len() > MAX_ATTRIBUTES
            || self.finding_refs.len() > MAX_REFERENCES
            || self.repair_refs.len() > MAX_REFERENCES
        {
            return Err("observe-store-corrupt:event-cardinality-limit".to_owned());
        }
        for (key, value) in &self.public_attributes {
            match privacy::sanitize_attribute(key, value)? {
                Some(sanitized) if sanitized == *value => {}
                _ => return Err("observe-store-corrupt:unredacted-sensitive-value".to_owned()),
            }
        }
        for key in &self.redacted_attribute_keys {
            privacy::validate_attribute_key(key)?;
            if privacy::is_claim_authority_key(key) || privacy::contains_sensitive_text(key) {
                return Err("observe-store-corrupt:invalid-redacted-attribute-key".to_owned());
            }
        }
        self.validate_size()
    }

    pub(super) fn sanitized_for_export(&self) -> Result<Self, String> {
        self.validate()?;
        Ok(self.clone())
    }

    pub(super) fn is_failure(&self) -> bool {
        matches!(
            self.outcome.as_str(),
            "fail" | "error" | "blocked" | "cancelled"
        )
    }

    pub(super) fn is_receipt_only(&self) -> bool {
        self.operation.starts_with("receipt.")
            || self.operation.starts_with("telemetry.")
            || self.source_id.contains("receipt")
            || self.source_id.contains("telemetry")
    }

    fn validate_size(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| "observe-event-serialization-failed".to_owned())?;
        if bytes.len() > MAX_ROW_BYTES {
            return Err("observe-row-limit: event exceeds the persisted row bound".to_owned());
        }
        Ok(())
    }
}
