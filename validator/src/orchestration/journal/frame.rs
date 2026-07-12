use super::super::{Binding, EventLog, OrchestrationError, OrchestrationEvent};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalHead {
    pub schema_version: String,
    pub binding: Binding,
    pub event_count: u64,
    pub last_event_id: String,
    pub log_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalSnapshot {
    pub head: JournalHead,
    pub log: EventLog,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalFrame {
    schema_version: String,
    event: OrchestrationEvent,
}

pub(crate) fn encode_log(log: &EventLog) -> Result<Vec<u8>, OrchestrationError> {
    if log.events().is_empty() || log.events().len() > 16_384 {
        return Err(OrchestrationError::JournalCorrupt);
    }
    let mut bytes = Vec::new();
    for event in log.events() {
        let frame = JournalFrame {
            schema_version: "OrchestrationJournalFrame-v1".to_owned(),
            event: event.clone(),
        };
        serde_json::to_writer(&mut bytes, &frame)
            .map_err(|_| OrchestrationError::JournalCorrupt)?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_JOURNAL_BYTES {
            return Err(OrchestrationError::ResourceLimit);
        }
    }
    Ok(bytes)
}

pub(crate) fn decode_log(bytes: &[u8]) -> Result<EventLog, OrchestrationError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_JOURNAL_BYTES || !bytes.ends_with(b"\n") {
        return Err(OrchestrationError::JournalCorrupt);
    }
    let mut events = Vec::new();
    for line in bytes[..bytes.len() - 1].split(|byte| *byte == b'\n') {
        if line.is_empty() {
            return Err(OrchestrationError::JournalCorrupt);
        }
        if events.len() >= 16_384 {
            return Err(OrchestrationError::ResourceLimit);
        }
        let frame: JournalFrame =
            serde_json::from_slice(line).map_err(|_| OrchestrationError::JournalCorrupt)?;
        if frame.schema_version != "OrchestrationJournalFrame-v1" {
            return Err(OrchestrationError::JournalCorrupt);
        }
        events.push(frame.event);
    }
    if events.is_empty() {
        return Err(OrchestrationError::JournalCorrupt);
    }
    let log = EventLog(events);
    validate_chain(&log)?;
    if encode_log(&log)? != bytes {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(log)
}

pub(crate) fn decode_head(bytes: &[u8]) -> Result<JournalHead, OrchestrationError> {
    let head: JournalHead =
        serde_json::from_slice(bytes).map_err(|_| OrchestrationError::JournalCorrupt)?;
    let mut canonical =
        serde_json::to_vec(&head).map_err(|_| OrchestrationError::JournalCorrupt)?;
    canonical.push(b'\n');
    if canonical != bytes {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(head)
}

fn validate_chain(log: &EventLog) -> Result<(), OrchestrationError> {
    let mut prior = None;
    let mut prior_tick = 0;
    for (index, event) in log.events().iter().enumerate() {
        event
            .verify_identity()
            .map_err(|_| OrchestrationError::JournalCorrupt)?;
        if event.sequence != index as u64
            || event.prior_event_id.as_deref() != prior
            || (index > 0 && event.logical_tick < prior_tick)
        {
            return Err(OrchestrationError::JournalCorrupt);
        }
        prior = Some(event.event_id.as_str());
        prior_tick = event.logical_tick;
    }
    Ok(())
}

pub(crate) fn head_for(
    binding: &Binding,
    log: &EventLog,
    bytes: &[u8],
) -> Result<JournalHead, OrchestrationError> {
    binding.validate()?;
    let last = log
        .events()
        .last()
        .ok_or(OrchestrationError::JournalCorrupt)?;
    Ok(JournalHead {
        schema_version: "OrchestrationJournalHead-v1".to_owned(),
        binding: binding.clone(),
        event_count: log.events().len() as u64,
        last_event_id: last.event_id.clone(),
        log_sha256: format!("sha256:{:x}", Sha256::digest(bytes)),
    })
}

pub(crate) fn validate_snapshot(
    head: &JournalHead,
    log: &EventLog,
    bytes: &[u8],
) -> Result<(), OrchestrationError> {
    if head.schema_version != "OrchestrationJournalHead-v1"
        || head != &head_for(&head.binding, log, bytes)?
        || encode_log(log)? != bytes
    {
        return Err(OrchestrationError::JournalCorrupt);
    }
    Ok(())
}
