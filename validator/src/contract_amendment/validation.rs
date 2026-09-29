use super::contracts::{CurrentAmendmentBinding, ValidatedCurrentAmendment};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA: &str = "harness-ultragoal.contract-amendment.v1";
const REQUIRED_CLAIMS: [&str; 14] = [
    "CL-SOURCE",
    "CL-PACKAGE",
    "CL-INSTALL",
    "CL-DISCOVERY",
    "CL-RUNTIME",
    "CL-FIT",
    "CL-ROUTINE",
    "CL-OBSERVABILITY",
    "CL-STRICT",
    "CL-ORCHESTRATION",
    "CL-EVAL-IMPROVEMENT",
    "CL-REAL-JOURNEY",
    "CL-RELEASE",
    "CL-COMPLETION",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AmendmentRow {
    schema: String,
    amendment_id: String,
    previous_contract_hash: String,
    new_contract_hash: String,
    previous_amendment_hash: String,
    amendment_hash: String,
    change_class: String,
    monotonicity: String,
    affected_claim_ids: Vec<String>,
    removed_or_weakened_claim_ids: Vec<String>,
    before_claim_ceiling: Vec<String>,
    after_claim_ceiling: Vec<String>,
    derived_removed_or_weakened_claim_ids: Vec<String>,
    derived_claim_delta_matches_declared: bool,
    approval: Approval,
    backlog_updates: Vec<ArtifactBinding>,
    created_at: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Approval {
    required: bool,
    status: String,
    artifact: Option<ArtifactBinding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactBinding {
    path: String,
    digest: String,
}

pub(crate) fn validate_current(
    bytes: &[u8],
    binding: CurrentAmendmentBinding<'_>,
) -> Result<ValidatedCurrentAmendment, &'static str> {
    let text = std::str::from_utf8(bytes).map_err(|_| "amendment_log_invalid")?;
    if text.is_empty() || !text.ends_with('\n') || text.lines().count() > 128 {
        return Err("amendment_log_invalid");
    }
    let values = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "amendment_log_invalid")?;
    let rows = values
        .iter()
        .cloned()
        .map(serde_json::from_value::<AmendmentRow>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "amendment_log_invalid")?;
    validate_rows(&values, &rows)?;
    let current = rows.last().ok_or("amendment_log_invalid")?;
    validate_current_binding(current, binding)?;
    Ok(ValidatedCurrentAmendment::new(
        current.amendment_id.clone(),
        current.amendment_hash.clone(),
    ))
}

fn validate_rows(values: &[Value], rows: &[AmendmentRow]) -> Result<(), &'static str> {
    let mut ids = BTreeSet::new();
    for (index, (value, row)) in values.iter().zip(rows).enumerate() {
        if !ids.insert(row.amendment_id.as_str())
            || amendment_number(&row.amendment_id) != index + 1
        {
            return Err("amendment_sequence_invalid");
        }
        if canonical_hash(value)? != row.amendment_hash || !valid_row(row) {
            return Err("amendment_semantics_invalid");
        }
        if index == 0 {
            if row.previous_amendment_hash
                != "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            {
                return Err("amendment_chain_invalid");
            }
        } else {
            let previous = &rows[index - 1];
            if row.previous_amendment_hash != previous.amendment_hash
                || row.previous_contract_hash != previous.new_contract_hash
            {
                return Err("amendment_chain_invalid");
            }
        }
    }
    Ok(())
}

fn valid_row(row: &AmendmentRow) -> bool {
    row.schema == SCHEMA
        && valid_hash(&row.previous_contract_hash)
        && valid_hash(&row.new_contract_hash)
        && valid_hash(&row.previous_amendment_hash)
        && valid_hash(&row.amendment_hash)
        && matches!(row.change_class.as_str(), "strengthens" | "clarifies")
        && row.monotonicity == "preserves_or_strengthens"
        && exact_claims(&row.affected_claim_ids)
        && exact_claims(&row.before_claim_ceiling)
        && exact_claims(&row.after_claim_ceiling)
        && row.removed_or_weakened_claim_ids.is_empty()
        && row.derived_removed_or_weakened_claim_ids.is_empty()
        && row.derived_claim_delta_matches_declared
        && !row.approval.required
        && row.approval.status == "not_required"
        && row.approval.artifact.is_none()
        && valid_artifacts(&row.backlog_updates)
        && !row.created_at.is_empty()
}

fn validate_current_binding(
    row: &AmendmentRow,
    binding: CurrentAmendmentBinding<'_>,
) -> Result<(), &'static str> {
    let backlog_matches = row.backlog_updates.len() == binding.backlog_updates.len()
        && row
            .backlog_updates
            .iter()
            .zip(binding.backlog_updates)
            .all(|(actual, expected)| {
                actual.path == expected.path && actual.digest == expected.digest
            });
    if row.amendment_id != binding.amendment_id
        || row.amendment_hash != binding.amendment_hash
        || row.previous_contract_hash != binding.previous_contract_hash
        || row.new_contract_hash != binding.new_contract_hash
        || row.change_class != binding.change_class
        || !backlog_matches
    {
        return Err("current_amendment_binding_invalid");
    }
    Ok(())
}

fn amendment_number(value: &str) -> usize {
    value
        .strip_prefix("AMEND-")
        .filter(|number| number.len() >= 3)
        .and_then(|number| number.parse().ok())
        .unwrap_or_default()
}

fn exact_claims(values: &[String]) -> bool {
    values.len() == REQUIRED_CLAIMS.len()
        && values.iter().map(String::as_str).collect::<BTreeSet<_>>()
            == REQUIRED_CLAIMS.into_iter().collect()
}

fn valid_artifacts(values: &[ArtifactBinding]) -> bool {
    !values.is_empty()
        && values.iter().all(|value| {
            !value.path.is_empty()
                && !value.path.starts_with('/')
                && !value.path.split('/').any(|part| part == "..")
                && valid_hash(&value.digest)
        })
        && values
            .iter()
            .map(|value| value.path.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            == values.len()
}

fn valid_hash(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn canonical_hash(value: &Value) -> Result<String, &'static str> {
    let mut canonical = value.clone();
    canonical
        .as_object_mut()
        .ok_or("amendment_log_invalid")?
        .remove("amendment_hash");
    let bytes = serde_json::to_vec(&canonical).map_err(|_| "amendment_log_invalid")?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
