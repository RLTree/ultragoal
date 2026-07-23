use super::store::{HostEffectRecord, HostLedger, read_ledger, write_ledger};
use super::{HostContext, digest_bytes};
use std::sync::Arc;

use super::super::{
    ConfinedMigrationEffect, EffectFault, EffectObservation, PlanDisposition,
    PlannedMigrationEffect,
};

const OBSERVATION_DOMAIN: &str = "harness-ultragoal.migration-host-effect-observation.v1";

pub(crate) struct DarwinMigrationEffects {
    context: Arc<HostContext>,
}

impl DarwinMigrationEffects {
    pub(super) fn new(context: Arc<HostContext>) -> Self {
        Self { context }
    }

    fn observe_from(
        &self,
        ledger: &HostLedger,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        let (authority, permit) = match ledger.effect(effect.effect_id()) {
            Some(record) => (
                record.authority().clone(),
                record.permit().map(ToOwned::to_owned),
            ),
            None => (effect.before().clone(), None),
        };
        let read_session = digest_bytes(
            format!(
                "{OBSERVATION_DOMAIN}|{}|{}|{}",
                ledger.digest(),
                effect.effect_id(),
                ledger
                    .effect(effect.effect_id())
                    .map(HostEffectRecord::revision)
                    .unwrap_or(0)
            )
            .as_bytes(),
        );
        EffectObservation::live(authority, read_session, permit)
            .map_err(|_| EffectFault::ambiguous("migration-host-effect-observation-invalid"))
    }

    fn validate_effect(
        effect: &PlannedMigrationEffect,
        permit: Option<&str>,
    ) -> Result<(), EffectFault> {
        if effect.validate().is_err()
            || effect.before().digest_sha256() != effect.after().digest_sha256()
        {
            return Err(EffectFault::rejected(
                "migration-host-effect-physical-preservation-refused",
            ));
        }
        match effect.disposition() {
            PlanDisposition::AdoptCompatibility => {
                if permit.is_none_or(|value| !super::super::super::valid_sha256(value)) {
                    return Err(EffectFault::rejected(
                        "migration-host-compatibility-permit-required",
                    ));
                }
            }
            PlanDisposition::RetireAuthority => {
                if permit.is_some() {
                    return Err(EffectFault::rejected(
                        "migration-host-retirement-permit-refused",
                    ));
                }
            }
            PlanDisposition::PendingMigration => {
                return Err(EffectFault::rejected(
                    "migration-host-pending-effect-refused",
                ));
            }
        }
        Ok(())
    }
}

impl ConfinedMigrationEffect for DarwinMigrationEffects {
    fn observe(
        &mut self,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        effect
            .validate()
            .map_err(|_| EffectFault::rejected("migration-host-effect-invalid"))?;
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| EffectFault::ambiguous("migration-host-lock-poisoned"))?;
        let ledger =
            read_ledger(&self.context).map_err(|error| EffectFault::ambiguous(error.code()))?;
        self.observe_from(&ledger, effect)
    }

    fn apply(
        &mut self,
        operation_id: &str,
        effect: &PlannedMigrationEffect,
        compatibility_effect_permit_sha256: Option<&str>,
    ) -> Result<EffectObservation, EffectFault> {
        if !super::super::super::valid_sha256(operation_id) {
            return Err(EffectFault::rejected(
                "migration-host-effect-operation-invalid",
            ));
        }
        Self::validate_effect(effect, compatibility_effect_permit_sha256)?;
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| EffectFault::ambiguous("migration-host-lock-poisoned"))?;
        let mut ledger =
            read_ledger(&self.context).map_err(|error| EffectFault::ambiguous(error.code()))?;
        if let Some(existing) = ledger.effect(effect.effect_id()) {
            if existing.authority() == effect.after()
                && existing.operation_id() == operation_id
                && existing.permit() == compatibility_effect_permit_sha256
            {
                return self.observe_from(&ledger, effect);
            }
            return Err(EffectFault::ambiguous(
                "migration-host-effect-state-conflict",
            ));
        }
        let record = HostEffectRecord::applied(
            effect.effect_id(),
            operation_id,
            effect.after().clone(),
            compatibility_effect_permit_sha256.map(ToOwned::to_owned),
            ledger
                .next_effect_revision(effect.effect_id())
                .map_err(|error| EffectFault::ambiguous(error.code()))?,
        );
        ledger.set_effect(record);
        ledger
            .record_apply()
            .map_err(|error| EffectFault::ambiguous(error.code()))?;
        ledger
            .advance()
            .map_err(|error| EffectFault::ambiguous(error.code()))?;
        write_ledger(&self.context, &ledger)
            .map_err(|error| EffectFault::ambiguous(error.code()))?;
        self.observe_from(&ledger, effect)
    }

    fn rollback(
        &mut self,
        operation_id: &str,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        if !super::super::super::valid_sha256(operation_id) || effect.validate().is_err() {
            return Err(EffectFault::rejected(
                "migration-host-rollback-input-invalid",
            ));
        }
        let _guard = self
            .context
            .io()
            .lock()
            .map_err(|_| EffectFault::ambiguous("migration-host-lock-poisoned"))?;
        let mut ledger =
            read_ledger(&self.context).map_err(|error| EffectFault::ambiguous(error.code()))?;
        let Some(existing) = ledger.effect(effect.effect_id()) else {
            return self.observe_from(&ledger, effect);
        };
        if existing.authority() != effect.after() || existing.operation_id() != operation_id {
            return Err(EffectFault::ambiguous(
                "migration-host-rollback-state-conflict",
            ));
        }
        ledger.remove_effect(effect.effect_id());
        ledger
            .record_rollback()
            .map_err(|error| EffectFault::ambiguous(error.code()))?;
        ledger
            .advance()
            .map_err(|error| EffectFault::ambiguous(error.code()))?;
        write_ledger(&self.context, &ledger)
            .map_err(|error| EffectFault::ambiguous(error.code()))?;
        self.observe_from(&ledger, effect)
    }
}
