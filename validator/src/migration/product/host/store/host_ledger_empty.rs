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
