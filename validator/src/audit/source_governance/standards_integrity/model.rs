use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnforcementRegistry {
    pub(super) schema: String,
    pub(super) rows: Vec<EnforcementRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EnforcementRow {
    pub(super) id: String,
    pub(super) source_law: String,
    pub(super) required_behavior: String,
    pub(super) enforcement_status: EnforcementStatus,
    pub(super) gate_or_fixture_path: String,
    pub(super) owner_lane: String,
    pub(super) claim_ids_affected: String,
    pub(super) current_status: String,
    pub(super) blocker_or_repair_action: String,
    pub(super) required_execplan_refs: Vec<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EnforcementStatus {
    Mechanized,
    Backlogged,
    Blocked,
    Informational,
}

impl EnforcementStatus {
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Mechanized => "mechanized",
            Self::Backlogged => "backlogged",
            Self::Blocked => "blocked",
            Self::Informational => "informational",
        }
    }
}

pub(super) struct AuditRow<'a> {
    pub(super) standard_id: &'a str,
    pub(super) audit_status: &'a str,
    pub(super) evidence_path: &'a str,
    pub(super) evidence_digest: &'a str,
    pub(super) audited_at: &'a str,
    pub(super) claim_ceiling_impact: &'a str,
}
