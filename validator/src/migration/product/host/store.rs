use super::{HostContext, HostError, LEDGER_NAME, MAX_HOST_FILE_BYTES, digest_bytes};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

use super::super::{
    AuthoritySnapshot, AuthorizationRecord, DurableMigrationStore, MigrationOperation,
    PlannedMigrationEffect, ReservationRequest, ReservationResult, StoreFault,
};

const LEDGER_SCHEMA: &str = "DarwinMigrationHostLedger-v1";
const LEDGER_DOMAIN: &str = "harness-ultragoal.migration-host-ledger.v1";
const MAX_LEDGER_ROWS: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostEffectRecord {
    schema_version: String,
    effect_id: String,
    operation_id: String,
    authority: AuthoritySnapshot,
    effect_permit_sha256: Option<String>,
    revision: u64,
}

impl HostEffectRecord {
    pub(super) fn applied(
        effect_id: &str,
        operation_id: &str,
        authority: AuthoritySnapshot,
        permit: Option<String>,
        revision: u64,
    ) -> Self {
        Self {
            schema_version: "DarwinMigrationHostEffect-v1".to_owned(),
            effect_id: effect_id.to_owned(),
            operation_id: operation_id.to_owned(),
            authority,
            effect_permit_sha256: permit,
            revision,
        }
    }

    pub(super) fn validate(&self, key: &str) -> bool {
        self.schema_version == "DarwinMigrationHostEffect-v1"
            && self.effect_id == key
            && super::super::super::valid_sha256(&self.effect_id)
            && super::super::super::valid_sha256(&self.operation_id)
            && self.authority.validate()
            && self
                .effect_permit_sha256
                .as_deref()
                .is_none_or(super::super::super::valid_sha256)
            && self.revision > 0
    }

    pub(super) fn authority(&self) -> &AuthoritySnapshot {
        &self.authority
    }

    pub(super) fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub(super) fn permit(&self) -> Option<&str> {
        self.effect_permit_sha256.as_deref()
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HostLedger {
    schema_version: String,
    scope_id: String,
    revision: u64,
    authorizations: BTreeMap<String, AuthorizationRecord>,
    consumed_authorizations: BTreeMap<String, String>,
    semantic_reservations: BTreeMap<String, String>,
    operations: BTreeMap<String, MigrationOperation>,
    effects: BTreeMap<String, HostEffectRecord>,
    effect_apply_count: u64,
    effect_rollback_count: u64,
    ledger_sha256: String,
}

impl HostLedger {
    pub(super) fn empty(scope_id: &str) -> Result<Self, HostError> {
        let mut value = Self {
            schema_version: LEDGER_SCHEMA.to_owned(),
            scope_id: scope_id.to_owned(),
            revision: 0,
            authorizations: BTreeMap::new(),
            consumed_authorizations: BTreeMap::new(),
            semantic_reservations: BTreeMap::new(),
            operations: BTreeMap::new(),
            effects: BTreeMap::new(),
            effect_apply_count: 0,
            effect_rollback_count: 0,
            ledger_sha256: String::new(),
        };
        value.refresh()?;
        Ok(value)
    }

    pub(super) fn canonical_bytes(&self) -> Result<Vec<u8>, HostError> {
        serde_json::to_vec(self).map_err(|_| HostError::new("migration-host-ledger-invalid"))
    }

    fn payload_digest(&self) -> Result<String, HostError> {
        let bytes = serde_json::to_vec(&(
            LEDGER_DOMAIN,
            &self.schema_version,
            &self.scope_id,
            self.revision,
            &self.authorizations,
            &self.consumed_authorizations,
            &self.semantic_reservations,
            &self.operations,
            &self.effects,
            self.effect_apply_count,
            self.effect_rollback_count,
        ))
        .map_err(|_| HostError::new("migration-host-ledger-invalid"))?;
        Ok(digest_bytes(&bytes))
    }

    fn refresh(&mut self) -> Result<(), HostError> {
        self.ledger_sha256 = self.payload_digest()?;
        Ok(())
    }

    pub(super) fn advance(&mut self) -> Result<(), HostError> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| HostError::new("migration-host-ledger-revision-exhausted"))?;
        self.refresh()
    }

    pub(super) fn validate(&self, scope_id: &str) -> Result<(), HostError> {
        if self.schema_version != LEDGER_SCHEMA
            || self.scope_id != scope_id
            || !super::super::super::valid_sha256(&self.scope_id)
            || self.authorizations.len() > MAX_LEDGER_ROWS
            || self.consumed_authorizations.len() > MAX_LEDGER_ROWS
            || self.semantic_reservations.len() > MAX_LEDGER_ROWS
            || self.operations.len() > MAX_LEDGER_ROWS
            || self.effects.len() > MAX_LEDGER_ROWS
            || self
                .authorizations
                .iter()
                .any(|(key, record)| key != record.authorization_id() || !record.validate_shape())
            || self.operations.iter().any(|(key, operation)| {
                key != operation.operation_id() || !operation.validate_shape()
            })
            || self
                .effects
                .iter()
                .any(|(key, record)| !record.validate(key))
            || self
                .consumed_authorizations
                .iter()
                .any(|(authorization, operation)| {
                    !self.authorizations.contains_key(authorization)
                        || !self.operations.contains_key(operation)
                })
            || self.semantic_reservations.iter().any(|(key, operation)| {
                !super::super::super::safe_reference(key)
                    || !self.operations.contains_key(operation)
            })
            || self.ledger_sha256 != self.payload_digest()?
        {
            return Err(HostError::new("migration-host-ledger-invalid"));
        }
        let mut envelopes = BTreeMap::new();
        for (operation_id, operation) in &self.operations {
            let envelope = operation_envelope(operation)?;
            let authorization_id = operation.authorization_id();
            if self.authorizations.get(authorization_id) != Some(&envelope.authorization)
                || self.consumed_authorizations.get(authorization_id) != Some(operation_id)
                || operation
                    .semantic_keys()
                    .iter()
                    .any(|key| self.semantic_reservations.get(key) != Some(operation_id))
            {
                return Err(HostError::new("migration-host-ledger-link-invalid"));
            }
            envelopes.insert(operation_id.as_str(), envelope);
        }
        for (semantic_key, operation_id) in &self.semantic_reservations {
            if self
                .operations
                .get(operation_id)
                .is_none_or(|operation| !operation.semantic_keys().contains(semantic_key))
            {
                return Err(HostError::new("migration-host-ledger-link-invalid"));
            }
        }
        for record in self.effects.values() {
            let Some(envelope) = envelopes.get(record.operation_id()) else {
                return Err(HostError::new("migration-host-ledger-link-invalid"));
            };
            if !envelope.effects.iter().any(|effect| {
                effect.effect_id() == record.effect_id && effect.after() == record.authority()
            }) {
                return Err(HostError::new("migration-host-ledger-link-invalid"));
            }
        }
        Ok(())
    }

    pub(super) fn effect(&self, effect_id: &str) -> Option<&HostEffectRecord> {
        self.effects.get(effect_id)
    }

    pub(super) fn set_effect(&mut self, record: HostEffectRecord) {
        self.effects.insert(record.effect_id.clone(), record);
    }

    pub(super) fn remove_effect(&mut self, effect_id: &str) {
        self.effects.remove(effect_id);
    }

    pub(super) fn next_effect_revision(&self, effect_id: &str) -> Result<u64, HostError> {
        self.effects
            .get(effect_id)
            .map(HostEffectRecord::revision)
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| HostError::new("migration-host-effect-revision-exhausted"))
    }

    pub(super) fn record_apply(&mut self) -> Result<(), HostError> {
        self.effect_apply_count = self
            .effect_apply_count
            .checked_add(1)
            .ok_or_else(|| HostError::new("migration-host-effect-count-exhausted"))?;
        Ok(())
    }

    pub(super) fn record_rollback(&mut self) -> Result<(), HostError> {
        self.effect_rollback_count = self
            .effect_rollback_count
            .checked_add(1)
            .ok_or_else(|| HostError::new("migration-host-effect-count-exhausted"))?;
        Ok(())
    }

    pub(super) fn counts(&self) -> (u64, u64) {
        (self.effect_apply_count, self.effect_rollback_count)
    }

    pub(super) fn digest(&self) -> &str {
        &self.ledger_sha256
    }
}

#[derive(Deserialize)]
struct DurableOperationEnvelope {
    authorization: AuthorizationRecord,
    effects: Vec<PlannedMigrationEffect>,
}

fn operation_envelope(
    operation: &MigrationOperation,
) -> Result<DurableOperationEnvelope, HostError> {
    let bytes = serde_json::to_vec(operation)
        .map_err(|_| HostError::new("migration-host-operation-invalid"))?;
    serde_json::from_slice(&bytes).map_err(|_| HostError::new("migration-host-operation-invalid"))
}

pub(crate) struct DarwinMigrationStore {
    context: Arc<HostContext>,
}

impl DarwinMigrationStore {
    pub(super) fn new(context: Arc<HostContext>) -> Result<Self, HostError> {
        let value = Self { context };
        let _guard = value
            .context
            .io()
            .lock()
            .map_err(|_| HostError::new("migration-host-lock-poisoned"))?;
        read_ledger(&value.context)?;
        drop(_guard);
        Ok(value)
    }

    #[cfg(test)]
    pub(crate) fn fail_on_cas_for_test(&self, number: usize) {
        *self
            .context
            .fail_on_cas
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(number);
        self.context
            .cas_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(crate) fn effect_counts_for_test(&self) -> (u64, u64) {
        let _guard = self.context.io().lock().unwrap();
        read_ledger(&self.context).unwrap().counts()
    }

    #[cfg(test)]
    pub(crate) fn only_operation_for_test(&self) -> MigrationOperation {
        let _guard = self.context.io().lock().unwrap();
        let ledger = read_ledger(&self.context).unwrap();
        assert_eq!(ledger.operations.len(), 1);
        ledger.operations.values().next().unwrap().clone()
    }
}

impl DurableMigrationStore for DarwinMigrationStore {
    fn register_authorization(&self, record: &AuthorizationRecord) -> Result<(), StoreFault> {
        if !record.validate_shape() {
            return Err(StoreFault::new("migration-host-authorization-invalid"));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let mut ledger = read_ledger(&self.context).map_err(store_fault)?;
        match ledger.authorizations.get(record.authorization_id()) {
            Some(existing) if existing == record => return Ok(()),
            Some(_) => return Err(StoreFault::new("migration-host-authorization-conflict")),
            None => {}
        }
        if ledger.authorizations.len() >= MAX_LEDGER_ROWS {
            return Err(StoreFault::new("migration-host-ledger-capacity-refused"));
        }
        ledger
            .authorizations
            .insert(record.authorization_id().to_owned(), record.clone());
        ledger.advance().map_err(store_fault)?;
        write_ledger(&self.context, &ledger).map_err(store_fault)
    }

    fn reserve_once(
        &self,
        request: &ReservationRequest,
        initial: &MigrationOperation,
    ) -> Result<ReservationResult, StoreFault> {
        if !request.validate_shape()
            || !initial.validate_shape()
            || request.operation_id() != initial.operation_id()
        {
            return Err(StoreFault::new("migration-host-reservation-invalid"));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let mut ledger = read_ledger(&self.context).map_err(store_fault)?;
        let registered = ledger
            .authorizations
            .get(request.authorization().authorization_id())
            .ok_or_else(|| StoreFault::new("migration-host-authorization-unregistered"))?;
        if registered != request.authorization() {
            return Err(StoreFault::new("migration-host-authorization-substituted"));
        }
        if let Some(operation_id) = ledger
            .consumed_authorizations
            .get(request.authorization().authorization_id())
        {
            return ledger
                .operations
                .get(operation_id)
                .cloned()
                .map(ReservationResult::Existing)
                .ok_or_else(|| StoreFault::new("migration-host-consumed-operation-missing"));
        }
        if request.semantic_keys().iter().any(|key| {
            ledger
                .semantic_reservations
                .get(key)
                .is_some_and(|owner| owner != request.operation_id())
        }) {
            return Err(StoreFault::new(
                "migration-host-semantic-reservation-conflict",
            ));
        }
        if ledger.operations.len() >= MAX_LEDGER_ROWS {
            return Err(StoreFault::new("migration-host-ledger-capacity-refused"));
        }
        ledger.consumed_authorizations.insert(
            request.authorization().authorization_id().to_owned(),
            request.operation_id().to_owned(),
        );
        for key in request.semantic_keys() {
            ledger
                .semantic_reservations
                .insert(key.clone(), request.operation_id().to_owned());
        }
        ledger
            .operations
            .insert(request.operation_id().to_owned(), initial.clone());
        ledger.advance().map_err(store_fault)?;
        write_ledger(&self.context, &ledger).map_err(store_fault)?;
        Ok(ReservationResult::Created(initial.clone()))
    }

    fn load_operation(&self, operation_id: &str) -> Result<Option<MigrationOperation>, StoreFault> {
        if !super::super::super::valid_sha256(operation_id) {
            return Err(StoreFault::new("migration-host-operation-id-invalid"));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let ledger = read_ledger(&self.context).map_err(store_fault)?;
        Ok(ledger.operations.get(operation_id).cloned())
    }

    fn compare_and_swap(
        &self,
        operation_id: &str,
        expected_revision: u64,
        expected_journal_sha256: &str,
        next: &MigrationOperation,
    ) -> Result<MigrationOperation, StoreFault> {
        #[cfg(test)]
        {
            let count = self
                .context
                .cas_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            let mut fail = self
                .context
                .fail_on_cas
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if *fail == Some(count) {
                *fail = None;
                return Err(StoreFault::new("migration-host-injected-interruption"));
            }
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| StoreFault::new("migration-host-lock-poisoned"))?;
        let mut ledger = read_ledger(&self.context).map_err(store_fault)?;
        let current = ledger
            .operations
            .get(operation_id)
            .ok_or_else(|| StoreFault::new("migration-host-operation-missing"))?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| StoreFault::new("migration-host-journal-revision-exhausted"))?;
        if current.revision() != expected_revision
            || current.journal_sha256() != expected_journal_sha256
            || next.operation_id() != operation_id
            || next.revision() != next_revision
            || !next.validate_shape()
        {
            return Err(StoreFault::new("migration-host-journal-cas-conflict"));
        }
        ledger
            .operations
            .insert(operation_id.to_owned(), next.clone());
        ledger.advance().map_err(store_fault)?;
        write_ledger(&self.context, &ledger).map_err(store_fault)?;
        Ok(next.clone())
    }
}

pub(super) fn read_ledger(context: &HostContext) -> Result<HostLedger, HostError> {
    context.verify_static()?;
    let observed = context
        .state()
        .read_regular(LEDGER_NAME, true, MAX_HOST_FILE_BYTES)?;
    let ledger: HostLedger = serde_json::from_slice(&observed.bytes)
        .map_err(|_| HostError::new("migration-host-ledger-invalid"))?;
    ledger.validate(context.scope_id())?;
    if ledger.canonical_bytes()? != observed.bytes {
        return Err(HostError::new("migration-host-ledger-noncanonical"));
    }
    context.verify_static()?;
    Ok(ledger)
}

pub(super) fn write_ledger(context: &HostContext, ledger: &HostLedger) -> Result<(), HostError> {
    ledger.validate(context.scope_id())?;
    let bytes = ledger.canonical_bytes()?;
    if bytes.len() as u64 > MAX_HOST_FILE_BYTES {
        return Err(HostError::new("migration-host-ledger-size-refused"));
    }
    context.state().write_atomic(LEDGER_NAME, &bytes)?;
    let reread = read_ledger(context)?;
    if &reread != ledger {
        return Err(HostError::new("migration-host-ledger-publish-substituted"));
    }
    Ok(())
}

fn store_fault(error: HostError) -> StoreFault {
    StoreFault::new(error.code())
}
