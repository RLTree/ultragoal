use serde::{Deserialize, Serialize};

pub(super) const POLICY_SCHEMA: &str = "harness-ultragoal.agent-standards-policy-shard.v1";
pub(super) const AUDIT_SCHEMA: &str = "harness-ultragoal.agent-standards-audit-shard.v1";
pub(super) const INDEX_SCHEMA: &str = "harness-ultragoal.agent-standards-enforcement.v1";

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyShard {
    schema: String,
    pub(super) rows: Vec<PolicyRow>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PolicyRow {
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

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EnforcementStatus {
    Mechanized,
    Backlogged,
    Blocked,
    Informational,
}

impl EnforcementStatus {
    pub(super) const fn text(self) -> &'static str {
        match self {
            Self::Mechanized => "mechanized",
            Self::Backlogged => "backlogged",
            Self::Blocked => "blocked",
            Self::Informational => "informational",
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditShard {
    schema: String,
    pub(super) rows: Vec<AuditRow>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuditRow {
    pub(super) standard_id: String,
    pub(super) audit_status: AuditStatus,
    pub(super) evidence_path: String,
    pub(super) evidence_digest: String,
    pub(super) audited_at: String,
    pub(super) claim_ceiling_impact: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AuditStatus {
    Pass,
    Fail,
    Pending,
    Blocked,
}

impl AuditStatus {
    pub(super) const fn text(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Pending => "pending",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Serialize)]
pub(super) struct PolicyIndex<'a> {
    pub(super) schema: &'static str,
    pub(super) rows: &'a [PolicyRow],
}

pub(super) fn parse_policy(bytes: &[u8]) -> Result<Vec<PolicyRow>, &'static str> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let shard = PolicyShard::deserialize(&mut deserializer).map_err(|_| "policy_json_invalid")?;
    deserializer.end().map_err(|_| "policy_trailing_data")?;
    if shard.schema != POLICY_SCHEMA || shard.rows.is_empty() {
        return Err("policy_shard_invalid");
    }
    for row in &shard.rows {
        let values = [
            row.id.as_str(),
            row.source_law.as_str(),
            row.required_behavior.as_str(),
            row.gate_or_fixture_path.as_str(),
            row.owner_lane.as_str(),
            row.claim_ids_affected.as_str(),
            row.current_status.as_str(),
            row.blocker_or_repair_action.as_str(),
        ];
        if values.iter().any(|value| !valid_text(value))
            || row.required_execplan_refs.is_empty()
            || row
                .required_execplan_refs
                .iter()
                .any(|value| !valid_text(value))
        {
            return Err("policy_row_invalid");
        }
    }
    Ok(shard.rows)
}

pub(super) fn parse_audit(bytes: &[u8]) -> Result<Vec<AuditRow>, &'static str> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let shard = AuditShard::deserialize(&mut deserializer).map_err(|_| "audit_json_invalid")?;
    deserializer.end().map_err(|_| "audit_trailing_data")?;
    if shard.schema != AUDIT_SCHEMA || shard.rows.is_empty() {
        return Err("audit_shard_invalid");
    }
    for row in &shard.rows {
        let values = [
            row.standard_id.as_str(),
            row.evidence_path.as_str(),
            row.evidence_digest.as_str(),
            row.audited_at.as_str(),
            row.claim_ceiling_impact.as_str(),
        ];
        if values.iter().any(|value| !valid_text(value)) {
            return Err("audit_row_invalid");
        }
    }
    Ok(shard.rows)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}
