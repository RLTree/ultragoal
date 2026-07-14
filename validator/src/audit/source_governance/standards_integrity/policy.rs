use super::model::{AuditRow, EnforcementRegistry, EnforcementRow, EnforcementStatus};
use super::registry_parser::{StandardsRegistryParseRequest, parse};
use crate::audit::source_governance::GovernedSource;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const JSON: &str = "agent-standards/enforcement.json";
const TSV: &str = "agent-standards/enforcement.tsv";
const AUDIT: &str = "agent-standards/enforcement-audit.tsv";
const JSON_SCHEMA: &str = "harness-ultragoal.agent-standards-enforcement.v1";
const TSV_HEADER: &str = "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\n";
const AUDIT_HEADER: &str =
    "standard_id\taudit_status\tevidence_path\tevidence_digest\taudited_at\tclaim_ceiling_impact";

pub(super) fn failures(root: &Path, sources: &[GovernedSource]) -> Vec<String> {
    let map = sources
        .iter()
        .map(|source| (source.relative.as_str(), source.bytes.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let mut failures = Vec::new();
    let Some(json) = map.get(JSON) else {
        return vec![format!(
            "standards_registry_integrity_surface_missing:{JSON}"
        )];
    };
    let registry = match parse(StandardsRegistryParseRequest { bytes: json }) {
        Ok(response) => response.registry,
        Err(_) => return vec!["standards_registry_integrity_json_invalid".to_string()],
    };
    failures.extend(registry_failures(root, &registry));
    match map.get(TSV) {
        Some(actual) if *actual == expected_tsv(&registry).as_bytes() => {}
        Some(_) => failures.push("standards_registry_integrity_index_drift".to_string()),
        None => failures.push(format!(
            "standards_registry_integrity_surface_missing:{TSV}"
        )),
    }
    match map.get(AUDIT) {
        Some(bytes) => failures.extend(audit_failures(root, bytes, &registry)),
        None => failures.push(format!(
            "standards_registry_integrity_surface_missing:{AUDIT}"
        )),
    }
    failures
}

fn registry_failures(root: &Path, registry: &EnforcementRegistry) -> Vec<String> {
    let mut failures = Vec::new();
    if registry.schema != JSON_SCHEMA {
        failures.push(format!(
            "standards_registry_integrity_schema_unknown:{}",
            registry.schema
        ));
    }
    let required = crate::audit::agent::standards::ids::REQUIRED_IDS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let actual = registry
        .rows
        .iter()
        .map(|row| row.id.as_str())
        .collect::<Vec<_>>();
    if actual.windows(2).any(|pair| pair[0] >= pair[1]) {
        failures.push("standards_registry_integrity_rows_noncanonical".to_string());
    }
    let actual_set = actual.iter().copied().collect::<BTreeSet<_>>();
    for missing in required.difference(&actual_set) {
        failures.push(format!(
            "standards_registry_integrity_required_id_missing:{missing}"
        ));
    }
    for unknown in actual_set.difference(&required) {
        failures.push(format!("standards_registry_integrity_unknown_id:{unknown}"));
    }
    for row in &registry.rows {
        failures.extend(row_failures(root, row));
    }
    failures
}

fn row_failures(root: &Path, row: &EnforcementRow) -> Vec<String> {
    let mut failures = Vec::new();
    let required_text = [
        row.source_law.as_str(),
        row.required_behavior.as_str(),
        row.owner_lane.as_str(),
        row.current_status.as_str(),
        row.blocker_or_repair_action.as_str(),
    ];
    if required_text.iter().any(|value| value.trim().is_empty())
        || !sorted_nonempty(&row.required_execplan_refs)
        || required_text
            .iter()
            .any(|value| value.contains(['\t', '\n', '\r']))
    {
        failures.push(format!(
            "standards_registry_integrity_row_invalid:{}",
            row.id
        ));
    }
    match row.enforcement_status {
        EnforcementStatus::Mechanized => {
            if row.gate_or_fixture_path.is_empty() || gate_missing(root, &row.gate_or_fixture_path)
            {
                failures.push(format!(
                    "standards_registry_integrity_gate_missing:{}",
                    row.id
                ));
            }
        }
        EnforcementStatus::Backlogged | EnforcementStatus::Blocked => {
            if row.blocker_or_repair_action.trim().is_empty() {
                failures.push(format!(
                    "standards_registry_integrity_debt_action_missing:{}",
                    row.id
                ));
            }
        }
        EnforcementStatus::Informational => {
            if row.claim_ids_affected != "none" {
                failures.push(format!(
                    "standards_registry_integrity_informational_overclaim:{}",
                    row.id
                ));
            }
        }
    }
    failures
}

fn expected_tsv(registry: &EnforcementRegistry) -> String {
    let mut output = String::from(TSV_HEADER);
    for row in &registry.rows {
        let fields = [
            row.id.as_str(),
            row.source_law.as_str(),
            row.required_behavior.as_str(),
            row.enforcement_status.text(),
            row.gate_or_fixture_path.as_str(),
            row.owner_lane.as_str(),
            row.claim_ids_affected.as_str(),
            row.current_status.as_str(),
            row.blocker_or_repair_action.as_str(),
        ];
        output.push_str(&fields.join("\t"));
        output.push('\t');
        output.push_str(&row.required_execplan_refs.join(";"));
        output.push('\n');
    }
    output
}

fn audit_failures(root: &Path, bytes: &[u8], registry: &EnforcementRegistry) -> Vec<String> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return vec!["standards_registry_integrity_audit_non_utf8".to_string()];
    };
    let mut lines = text.lines();
    if lines.next() != Some(AUDIT_HEADER) {
        return vec!["standards_registry_integrity_audit_header_invalid".to_string()];
    }
    let mut failures = Vec::new();
    let mut audits = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let cells = line.split('\t').collect::<Vec<_>>();
        if cells.len() != 6 {
            failures.push("standards_registry_integrity_audit_row_invalid".to_string());
            continue;
        }
        let row = AuditRow {
            standard_id: cells[0],
            audit_status: cells[1],
            evidence_path: cells[2],
            evidence_digest: cells[3],
            audited_at: cells[4],
            claim_ceiling_impact: cells[5],
        };
        if audits.insert(row.standard_id, row).is_some() {
            failures.push(format!(
                "standards_registry_integrity_audit_duplicate:{}",
                cells[0]
            ));
        }
    }
    let ids = registry
        .rows
        .iter()
        .map(|row| row.id.as_str())
        .collect::<BTreeSet<_>>();
    for id in &ids {
        if !audits.contains_key(id) {
            failures.push(format!("standards_registry_integrity_audit_missing:{id}"));
        }
    }
    for (id, audit) in audits {
        if !ids.contains(id) {
            failures.push(format!("standards_registry_integrity_audit_unknown:{id}"));
        }
        if audit.audited_at.is_empty() || audit.claim_ceiling_impact.is_empty() {
            failures.push(format!(
                "standards_registry_integrity_audit_metadata_missing:{id}"
            ));
        }
        if audit.audit_status == "pass" && evidence_invalid(root, &audit) {
            failures.push(format!(
                "standards_registry_integrity_audit_evidence_invalid:{id}"
            ));
        } else if !matches!(
            audit.audit_status,
            "pass" | "fail" | "pending" | "blocked" | "not_run"
        ) {
            failures.push(format!(
                "standards_registry_integrity_audit_status_unknown:{id}"
            ));
        }
    }
    failures
}

fn evidence_invalid(root: &Path, row: &AuditRow<'_>) -> bool {
    let path = root.join(row.evidence_path);
    row.evidence_digest == crate::digest::ZERO
        || !row.evidence_digest.starts_with("sha256:")
        || crate::digest::file(&path).map_or(true, |digest| digest != row.evidence_digest)
}

fn gate_missing(root: &Path, relative: &str) -> bool {
    let direct = root.join(relative);
    if crate::digest::read_file_bytes(&direct).is_ok() {
        return false;
    }
    relative.strip_prefix("scripts/").is_none_or(|tail| {
        crate::digest::read_file_bytes(&root.join("templates/scripts").join(tail)).is_err()
    })
}

fn sorted_nonempty(values: &[String]) -> bool {
    !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
}
