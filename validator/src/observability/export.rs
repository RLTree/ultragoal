use super::privacy;
use super::{EventQuery, EventStore, SemanticEvent};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

/// Explicit optional external boundary. Implementations own transport; the store owns policy.
pub trait ExportAdapter {
    fn adapter_id(&self) -> &str;
    fn bound_candidate_id(&self) -> &str;
    fn is_available(&self) -> bool;
    fn accepts_redacted_only(&self) -> bool;
    fn export(
        &mut self,
        events: &[SemanticEvent],
        timeout: Duration,
    ) -> Result<Vec<String>, String>;
    fn reconcile(
        &mut self,
        candidate_id: &str,
        event_ids: &[String],
        timeout: Duration,
    ) -> Result<Vec<String>, String>;
}

pub struct ExplicitExportRequest<'a> {
    pub query: &'a EventQuery,
    pub configured: bool,
    pub consent_granted: bool,
    pub timeout: Duration,
    pub adapter: Option<&'a mut dyn ExportAdapter>,
}

impl EventStore {
    pub fn export_explicit(&self, request: ExplicitExportRequest<'_>) -> Result<usize, String> {
        if !request.configured {
            return Err("observe-export-disabled: explicit configuration is required".to_owned());
        }
        if !request.consent_granted {
            return Err("observe-export-consent-denied: explicit consent is required".to_owned());
        }
        if request.timeout.is_zero() || request.timeout > Duration::from_secs(60) {
            return Err("observe-export-timeout-invalid".to_owned());
        }
        let adapter = request
            .adapter
            .ok_or_else(|| "observe-export-adapter-absent".to_owned())?;
        privacy::validate_identifier("export-adapter-id", adapter.adapter_id())?;
        if !adapter.is_available() {
            return Err("observe-export-adapter-unavailable".to_owned());
        }
        if !adapter.accepts_redacted_only() {
            return Err("observe-export-adapter-redaction-contract-denied".to_owned());
        }
        if adapter.bound_candidate_id() != self.candidate_id {
            return Err("observe-export-wrong-candidate".to_owned());
        }
        let events = self.query(request.query)?;
        let safe_events = events
            .iter()
            .map(SemanticEvent::sanitized_for_export)
            .collect::<Result<Vec<_>, _>>()?;
        verify_export_payload(&safe_events)?;
        let ids: Vec<String> = safe_events
            .iter()
            .map(|event| event.event_id().to_owned())
            .collect();
        if ids.is_empty() {
            return Ok(0);
        }
        let started = Instant::now();
        let accepted = adapter
            .export(&safe_events, request.timeout)
            .map_err(|_| "observe-export-adapter-outage".to_owned())?;
        ensure_within_timeout(started, request.timeout)?;
        reconcile_ids(&ids, &accepted, "acceptance")?;
        let remaining = request
            .timeout
            .checked_sub(started.elapsed())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| "observe-export-timeout".to_owned())?;
        let acknowledged = adapter
            .reconcile(&self.candidate_id, &ids, remaining)
            .map_err(|_| "observe-export-reconciliation-outage".to_owned())?;
        ensure_within_timeout(started, request.timeout)?;
        reconcile_ids(&ids, &acknowledged, "roundtrip")?;
        Ok(ids.len())
    }
}

fn verify_export_payload(events: &[SemanticEvent]) -> Result<(), String> {
    for event in events {
        let bytes = serde_json::to_vec(event)
            .map_err(|_| "observe-export-serialization-failed".to_owned())?;
        let text = std::str::from_utf8(&bytes)
            .map_err(|_| "observe-export-serialization-failed".to_owned())?;
        if privacy::contains_sensitive_text(text) {
            return Err("observe-export-unredacted-payload-rejected".to_owned());
        }
    }
    Ok(())
}

fn reconcile_ids(expected: &[String], actual: &[String], stage: &str) -> Result<(), String> {
    let unique: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    if unique.len() != actual.len() {
        return Err(format!("observe-export-{stage}-duplicate-ack"));
    }
    let expected: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    if unique != expected {
        return Err(format!("observe-export-{stage}-partial-or-wrong-ack"));
    }
    Ok(())
}

fn ensure_within_timeout(started: Instant, timeout: Duration) -> Result<(), String> {
    if started.elapsed() > timeout {
        return Err("observe-export-timeout".to_owned());
    }
    Ok(())
}
