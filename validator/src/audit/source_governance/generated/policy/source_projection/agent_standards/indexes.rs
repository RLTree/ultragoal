use super::ProjectionRequest;
use super::contracts::{AuditRow, INDEX_SCHEMA, PolicyIndex, PolicyRow, parse_audit, parse_policy};
use std::collections::BTreeSet;

const POLICY_JSON_OUTPUTS: &[&str] = &[
    "agent-standards/enforcement.json",
    "templates/agent-standards/enforcement.json",
];
const POLICY_TSV_OUTPUTS: &[&str] = &[
    "agent-standards/enforcement.tsv",
    "templates/agent-standards/enforcement.tsv",
];
const AUDIT_TSV_OUTPUTS: &[&str] = &[
    "agent-standards/enforcement-audit.tsv",
    "templates/agent-standards/enforcement-audit.tsv",
];
const POLICY_HEADER: &str = "id\tsource_law\trequired_behavior\tenforcement_status\tgate_or_fixture_path\towner_lane\tclaim_ids_affected\tcurrent_status\tblocker_or_repair_action\trequired_execplan_refs\n";
const AUDIT_HEADER: &str =
    "standard_id\taudit_status\tevidence_path\tevidence_digest\taudited_at\tclaim_ceiling_impact\n";

pub(super) fn owns(output: &str) -> bool {
    POLICY_JSON_OUTPUTS.contains(&output)
        || POLICY_TSV_OUTPUTS.contains(&output)
        || AUDIT_TSV_OUTPUTS.contains(&output)
}

pub(super) fn render(request: ProjectionRequest<'_>) -> Result<Vec<u8>, &'static str> {
    if POLICY_JSON_OUTPUTS.contains(&request.output) {
        let rows = policy_rows(&request)?;
        let mut bytes = serde_json::to_vec_pretty(&PolicyIndex {
            schema: INDEX_SCHEMA,
            rows: &rows,
        })
        .map_err(|_| "policy_index_render_failed")?;
        bytes.push(b'\n');
        return Ok(bytes);
    }
    if POLICY_TSV_OUTPUTS.contains(&request.output) {
        return Ok(policy_tsv(policy_rows(&request)?).into_bytes());
    }
    if AUDIT_TSV_OUTPUTS.contains(&request.output) {
        return Ok(audit_tsv(audit_rows(&request)?).into_bytes());
    }
    Err("index_output_unsupported")
}

fn policy_rows(request: &ProjectionRequest<'_>) -> Result<Vec<PolicyRow>, &'static str> {
    require_complete_source_set(request, "agent-standards/policy/")?;
    let mut rows = Vec::new();
    for source in request.canonical_sources {
        let bytes = &request
            .sources
            .get(source.as_str())
            .ok_or("policy_source_missing")?
            .bytes;
        rows.extend(parse_policy(bytes)?);
    }
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    if rows.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err("policy_identity_duplicate");
    }
    Ok(rows)
}

fn audit_rows(request: &ProjectionRequest<'_>) -> Result<Vec<AuditRow>, &'static str> {
    require_complete_source_set(request, "agent-standards/audit/")?;
    let mut rows = Vec::new();
    for source in request.canonical_sources {
        let bytes = &request
            .sources
            .get(source.as_str())
            .ok_or("audit_source_missing")?
            .bytes;
        rows.extend(parse_audit(bytes)?);
    }
    rows.sort_by(|left, right| left.standard_id.cmp(&right.standard_id));
    if rows
        .windows(2)
        .any(|pair| pair[0].standard_id == pair[1].standard_id)
    {
        return Err("audit_identity_duplicate");
    }
    Ok(rows)
}

fn require_complete_source_set(
    request: &ProjectionRequest<'_>,
    prefix: &str,
) -> Result<(), &'static str> {
    let declared = request
        .canonical_sources
        .iter()
        .map(|source| source.as_str())
        .collect::<BTreeSet<_>>();
    let actual = request
        .sources
        .keys()
        .copied()
        .filter(|path| {
            path.strip_prefix(prefix)
                .is_some_and(|tail| tail.ends_with(".json") && !tail.contains('/'))
        })
        .collect::<BTreeSet<_>>();
    if declared.is_empty() || declared != actual {
        Err("canonical_source_set_incomplete")
    } else {
        Ok(())
    }
}

fn policy_tsv(rows: Vec<PolicyRow>) -> String {
    let mut output = String::from(POLICY_HEADER);
    for row in rows {
        append_row(
            &mut output,
            &[
                row.id,
                row.source_law,
                row.required_behavior,
                row.enforcement_status.text().to_string(),
                row.gate_or_fixture_path,
                row.owner_lane,
                row.claim_ids_affected,
                row.current_status,
                row.blocker_or_repair_action,
                row.required_execplan_refs.join(";"),
            ],
        );
    }
    output
}

fn audit_tsv(rows: Vec<AuditRow>) -> String {
    let mut output = String::from(AUDIT_HEADER);
    for row in rows {
        append_row(
            &mut output,
            &[
                row.standard_id,
                row.audit_status.text().to_string(),
                row.evidence_path,
                row.evidence_digest,
                row.audited_at,
                row.claim_ceiling_impact,
            ],
        );
    }
    output
}

fn append_row(output: &mut String, fields: &[String]) {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            output.push('\t');
        }
        output.push_str(&escaped_cell(field));
    }
    output.push('\n');
}

fn escaped_cell(value: &str) -> String {
    if value.contains(['\t', '\r', '\n', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}
