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
    let mut physical_duplicate_count = 0_usize;
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
            Some(existing) if existing == &row.event => physical_duplicate_count += 1,
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
    let physical_row_count = by_id.len() + physical_duplicate_count;
    Ok(DecodedRows {
        events: by_id.into_values().collect(),
        physical_row_count,
    })
}
