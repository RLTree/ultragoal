use super::SemanticEvent;
use super::limits::{MAX_ROW_BYTES, ROW_SCHEMA};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRow {
    row_version: String,
    event: SemanticEvent,
    checksum_sha256: String,
}

pub(super) struct DecodedRows {
    pub events: Vec<SemanticEvent>,
    pub physical_row_count: usize,
}

pub(super) struct ConflictTolerantDecodedRows {
    pub events: Vec<SemanticEvent>,
    pub physical_row_count: usize,
    pub duplicate_or_conflicting_event_id: bool,
}

pub(super) fn encode(event: &SemanticEvent) -> Result<Vec<u8>, String> {
    event.validate()?;
    let event_bytes =
        serde_json::to_vec(event).map_err(|_| "observe-event-serialization-failed".to_owned())?;
    let row = StoredRow {
        row_version: ROW_SCHEMA.to_owned(),
        checksum_sha256: format!("sha256:{:x}", Sha256::digest(&event_bytes)),
        event: event.clone(),
    };
    let mut bytes =
        serde_json::to_vec(&row).map_err(|_| "observe-row-serialization-failed".to_owned())?;
    bytes.push(b'\n');
    if bytes.len() > MAX_ROW_BYTES {
        return Err("observe-row-limit: encoded row exceeds the byte bound".to_owned());
    }
    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8], max_scan_rows: usize) -> Result<DecodedRows, String> {
    if bytes.is_empty() {
        return Ok(DecodedRows {
            events: Vec::new(),
            physical_row_count: 0,
        });
    }
    if !bytes.ends_with(b"\n") {
        return Err("observe-store-corrupt:truncated-tail".to_owned());
    }
    let mut by_id = BTreeMap::<String, SemanticEvent>::new();
    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        let row_number = index + 1;
        if row_number > max_scan_rows {
            return Err("observe-scan-limit: row scan bound exceeded".to_owned());
        }
        if line.is_empty() || line.len() + 1 > MAX_ROW_BYTES {
            return Err(format!("observe-store-corrupt:row-{row_number}-size"));
        }
        let row: StoredRow = serde_json::from_slice(line)
            .map_err(|_| format!("observe-store-corrupt:row-{row_number}-malformed-or-unknown"))?;
        if row.row_version != ROW_SCHEMA {
            return Err(format!(
                "observe-store-corrupt:row-{row_number}-unsupported-version"
            ));
        }
        row.event
            .validate()
            .map_err(|_| format!("observe-store-corrupt:row-{row_number}-invalid-event"))?;
        let event_bytes = serde_json::to_vec(&row.event)
            .map_err(|_| format!("observe-store-corrupt:row-{row_number}-invalid-event"))?;
        let expected = format!("sha256:{:x}", Sha256::digest(&event_bytes));
        if row.checksum_sha256 != expected {
            return Err(format!("observe-store-corrupt:row-{row_number}-checksum"));
        }
        match by_id.get(row.event.event_id()) {
            Some(existing) if existing == &row.event => {
                return Err(format!(
                    "observe-store-corrupt:row-{row_number}-duplicate-event"
                ));
            }
            Some(_) => {
                return Err(format!(
                    "observe-store-corrupt:row-{row_number}-conflicting-event-id"
                ));
            }
            None => {
                by_id.insert(row.event.event_id().to_owned(), row.event);
            }
        }
    }
    let physical_row_count = by_id.len();
    Ok(DecodedRows {
        events: by_id.into_values().collect(),
        physical_row_count,
    })
}

pub(super) fn decode_conflict_tolerant(
    bytes: &[u8],
    max_scan_rows: usize,
) -> Result<ConflictTolerantDecodedRows, String> {
    if bytes.is_empty() {
        return Ok(ConflictTolerantDecodedRows {
            events: Vec::new(),
            physical_row_count: 0,
            duplicate_or_conflicting_event_id: false,
        });
    }
    if !bytes.ends_with(b"\n") {
        return Err("observe-store-corrupt:truncated-tail".to_owned());
    }
    let mut events = Vec::new();
    let mut by_id = BTreeMap::<String, SemanticEvent>::new();
    let mut duplicate_or_conflicting_event_id = false;
    for (index, line) in bytes[..bytes.len() - 1]
        .split(|byte| *byte == b'\n')
        .enumerate()
    {
        let row_number = index + 1;
        if row_number > max_scan_rows {
            return Err("observe-scan-limit: row scan bound exceeded".to_owned());
        }
        if line.is_empty() || line.len() + 1 > MAX_ROW_BYTES {
            return Err(format!("observe-store-corrupt:row-{row_number}-size"));
        }
        let row: StoredRow = serde_json::from_slice(line)
            .map_err(|_| format!("observe-store-corrupt:row-{row_number}-malformed-or-unknown"))?;
        if row.row_version != ROW_SCHEMA {
            return Err(format!(
                "observe-store-corrupt:row-{row_number}-unsupported-version"
            ));
        }
        row.event
            .validate()
            .map_err(|_| format!("observe-store-corrupt:row-{row_number}-invalid-event"))?;
        let event_bytes = serde_json::to_vec(&row.event)
            .map_err(|_| format!("observe-store-corrupt:row-{row_number}-invalid-event"))?;
        let expected = format!("sha256:{:x}", Sha256::digest(&event_bytes));
        if row.checksum_sha256 != expected {
            return Err(format!("observe-store-corrupt:row-{row_number}-checksum"));
        }
        if by_id.contains_key(row.event.event_id()) {
            duplicate_or_conflicting_event_id = true;
        } else {
            by_id.insert(row.event.event_id().to_owned(), row.event.clone());
        }
        events.push(row.event);
    }
    Ok(ConflictTolerantDecodedRows {
        physical_row_count: events.len(),
        events,
        duplicate_or_conflicting_event_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::SemanticEventInput;

    fn event(id: &str) -> SemanticEvent {
        SemanticEvent::new(SemanticEventInput {
            context_id: "context".to_owned(),
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
    fn strict_decode_still_rejects_a_duplicate_row_while_tolerant_decode_records_it() {
        let row = encode(&event("duplicate")).unwrap();
        let bytes = [row.as_slice(), row.as_slice()].concat();
        assert_eq!(
            decode(&bytes, 2).err().unwrap(),
            "observe-store-corrupt:row-2-duplicate-event"
        );
        let decoded = decode_conflict_tolerant(&bytes, 2).unwrap();
        assert_eq!(decoded.physical_row_count, 2);
        assert_eq!(decoded.events.len(), 2);
        assert!(decoded.duplicate_or_conflicting_event_id);
    }

    #[test]
    fn tolerant_decode_scans_past_a_duplicate_and_rejects_later_corruption() {
        let row = encode(&event("duplicate")).unwrap();
        let malformed = b"{not-json}\n";
        let malformed_bytes = [row.as_slice(), row.as_slice(), malformed].concat();
        assert_eq!(
            decode_conflict_tolerant(&malformed_bytes, 3).err().unwrap(),
            "observe-store-corrupt:row-3-malformed-or-unknown"
        );

        let mut bad_checksum: StoredRow = serde_json::from_slice(&row[..row.len() - 1]).unwrap();
        bad_checksum.checksum_sha256 = format!("sha256:{}", "0".repeat(64));
        let mut bad_checksum = serde_json::to_vec(&bad_checksum).unwrap();
        bad_checksum.push(b'\n');
        let checksum_bytes = [row.as_slice(), row.as_slice(), bad_checksum.as_slice()].concat();
        assert_eq!(
            decode_conflict_tolerant(&checksum_bytes, 3).err().unwrap(),
            "observe-store-corrupt:row-3-checksum"
        );
    }

    #[test]
    fn tolerant_decode_records_conflicting_content_for_one_event_id() {
        let first = event("conflicting");
        let mut second = first.clone();
        second.add_public_attribute("different", "true").unwrap();
        let first_row = encode(&first).unwrap();
        let second_row = encode(&second).unwrap();
        let decoded =
            decode_conflict_tolerant(&[first_row.as_slice(), second_row.as_slice()].concat(), 2)
                .unwrap();
        assert_eq!(decoded.events, vec![first, second]);
        assert!(decoded.duplicate_or_conflicting_event_id);
    }
}
