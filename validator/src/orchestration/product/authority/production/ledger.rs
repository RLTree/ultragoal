use super::{
    ProductError, ReservedExecution, ValidatedExecution, checkpoint::LedgerCheckpoint, store::Store,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Mutex;

const RECORD_SCHEMA: &str = "OrchestrationPermitReplayRecord-v1";
const RECORD_DOMAIN: &[u8] = b"harness-ultragoal/orchestration-permit-replay/v1\0";
const MAX_RECORDS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LedgerState {
    Issued,
    Reserved,
    Committed,
    Refused,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LedgerRecord {
    schema_version: String,
    sequence: u64,
    permit_id: String,
    causal_slot_id: String,
    state: LedgerState,
    prior_record_id: Option<String>,
    record_id: String,
    authenticator: String,
}

pub(super) struct Ledger {
    store: Store,
    checkpoint: Mutex<LedgerCheckpoint>,
}

impl Ledger {
    pub(super) fn issue(&self, permit_id: &str, causal_slot_id: &str) -> Result<(), ProductError> {
        self.transition(permit_id, Some(causal_slot_id), None, LedgerState::Issued)
    }

    pub(super) fn reserve<'a>(
        &self,
        execution: ValidatedExecution<'a>,
    ) -> Result<ReservedExecution<'a>, ProductError> {
        let (authority, operation, permit_id) = execution.into_parts();
        self.transition(
            &permit_id,
            None,
            Some(LedgerState::Issued),
            LedgerState::Reserved,
        )?;
        Ok(ReservedExecution::new(authority, operation, permit_id))
    }

    pub(super) fn complete<T>(
        &self,
        reservation: ReservedExecution<'_>,
        operation: impl FnOnce() -> Result<T, ProductError>,
    ) -> Result<T, ProductError> {
        let permit_id = reservation.permit_id();
        match operation() {
            Ok(outcome) => {
                self.transition(
                    permit_id,
                    None,
                    Some(LedgerState::Reserved),
                    LedgerState::Committed,
                )?;
                Ok(outcome)
            }
            Err(error) => {
                self.transition(
                    permit_id,
                    None,
                    Some(LedgerState::Reserved),
                    LedgerState::Ambiguous,
                )?;
                Err(error)
            }
        }
    }

    pub(super) fn reconcile(
        &self,
        permit_id: &str,
        state: LedgerState,
    ) -> Result<(), ProductError> {
        if !matches!(
            state,
            LedgerState::Committed | LedgerState::Refused | LedgerState::Ambiguous
        ) {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        self.transition(permit_id, None, Some(LedgerState::Reserved), state)
    }

    pub(super) fn state(&self, permit_id: &str) -> Result<Option<LedgerState>, ProductError> {
        let _lock = self.store.lock()?;
        self.store.verify()?;
        let records = self.read_records()?;
        self.require_current_checkpoint(&records)?;
        Ok(latest_states(&records).get(permit_id).copied())
    }

    #[cfg(test)]
    pub(super) fn reserve_for_test(&self, permit_id: &str) -> Result<(), ProductError> {
        self.transition(
            permit_id,
            None,
            Some(LedgerState::Issued),
            LedgerState::Reserved,
        )
    }

    fn transition(
        &self,
        permit_id: &str,
        new_causal_slot_id: Option<&str>,
        expected: Option<LedgerState>,
        next: LedgerState,
    ) -> Result<(), ProductError> {
        validate_digest(permit_id)?;
        let _lock = self.store.lock()?;
        self.store.verify()?;
        let mut records = self.read_records()?;
        self.require_current_checkpoint(&records)?;
        require_state(&records, permit_id, expected)?;
        let causal_slot_id = match new_causal_slot_id {
            Some(value) => {
                validate_digest(value)?;
                require_available_causal_slot(&records, value)?;
                value.to_owned()
            }
            None => records
                .iter()
                .rev()
                .find(|record| record.permit_id == permit_id)
                .map(|record| record.causal_slot_id.clone())
                .ok_or(ProductError::AuthorityStoreInvalid)?,
        };
        self.append(&mut records, permit_id, &causal_slot_id, next)
    }

    fn append(
        &self,
        records: &mut Vec<LedgerRecord>,
        permit_id: &str,
        causal_slot_id: &str,
        next: LedgerState,
    ) -> Result<(), ProductError> {
        if records.len() >= MAX_RECORDS {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        let mut record = LedgerRecord {
            schema_version: RECORD_SCHEMA.to_owned(),
            sequence: records.len() as u64 + 1,
            permit_id: permit_id.to_owned(),
            causal_slot_id: causal_slot_id.to_owned(),
            state: next,
            prior_record_id: records.last().map(|record| record.record_id.clone()),
            record_id: String::new(),
            authenticator: String::new(),
        };
        record.record_id = record_id(&record)?;
        record.authenticator = authenticate(self.store.key(), &record)?;
        self.store.append_record(
            &serde_json::to_vec(&record).map_err(|_| ProductError::AuthorityStoreInvalid)?,
        )?;
        records.push(record);
        *self.checkpoint_lock()? = checkpoint_for(records, self.store.ledger_stamp()?);
        Ok(())
    }

    fn read_records(&self) -> Result<Vec<LedgerRecord>, ProductError> {
        let bytes = self.store.read_ledger()?;
        let mut records = Vec::new();
        if bytes.is_empty() {
            return Ok(records);
        }
        if !bytes.ends_with(b"\n") {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        for line in bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            if records.len() >= MAX_RECORDS {
                return Err(ProductError::AuthorityStoreInvalid);
            }
            let record: LedgerRecord =
                serde_json::from_slice(line).map_err(|_| ProductError::AuthorityStoreInvalid)?;
            validate_record(self.store.key(), &records, &record)?;
            records.push(record);
        }
        Ok(records)
    }
}

include!("ledger_checkpoint.rs");
include!("ledger_record.rs");

fn require_state(
    records: &[LedgerRecord],
    permit_id: &str,
    expected: Option<LedgerState>,
) -> Result<(), ProductError> {
    let current = latest_states(records).get(permit_id).copied();
    if current == expected {
        return Ok(());
    }
    Err(match current {
        Some(LedgerState::Reserved) => ProductError::AuthorityReplayPending,
        Some(_) => ProductError::AuthorityReplay,
        None => ProductError::AuthorityInvalid,
    })
}

fn latest_states(records: &[LedgerRecord]) -> BTreeMap<String, LedgerState> {
    records
        .iter()
        .map(|record| (record.permit_id.clone(), record.state))
        .collect()
}

fn require_available_causal_slot(
    records: &[LedgerRecord],
    causal_slot_id: &str,
) -> Result<(), ProductError> {
    let states = latest_states(records);
    let conflict = records.iter().rev().find(|record| {
        record.causal_slot_id == causal_slot_id
            && states.get(&record.permit_id) != Some(&LedgerState::Refused)
    });
    match conflict.map(|record| record.state) {
        None => Ok(()),
        Some(LedgerState::Reserved) => Err(ProductError::AuthorityReplayPending),
        Some(_) => Err(ProductError::AuthorityReplay),
    }
}
