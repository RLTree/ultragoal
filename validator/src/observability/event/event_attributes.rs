use super::*;

impl SemanticEvent {
    pub fn redacted_attribute_keys(&self) -> &BTreeSet<String> {
        &self.redacted_attribute_keys
    }
    pub fn finding_refs(&self) -> &BTreeSet<String> {
        &self.finding_refs
    }
    pub fn repair_refs(&self) -> &BTreeSet<String> {
        &self.repair_refs
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
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
    pub(crate) fn sanitized_for_export(&self) -> Result<Self, String> {
        self.validate()?;
        Ok(self.clone())
    }
    pub(crate) fn is_failure(&self) -> bool {
        matches!(
            self.outcome.as_str(),
            "fail" | "error" | "blocked" | "cancelled"
        )
    }
    pub(crate) fn is_receipt_only(&self) -> bool {
        self.operation.starts_with("receipt.")
            || self.operation.starts_with("telemetry.")
            || self.source_id.contains("receipt")
            || self.source_id.contains("telemetry")
    }
    pub(crate) fn validate_size(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| "observe-event-serialization-failed".to_owned())?;
        if bytes.len() > MAX_ROW_BYTES {
            return Err("observe-row-limit: event exceeds the persisted row bound".to_owned());
        }
        Ok(())
    }
}
