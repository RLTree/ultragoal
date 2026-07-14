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
