mod debt;
mod overlap;
mod record;
mod root;

pub(super) fn protected_path_match(normalized: &str, pattern: &str) -> bool {
    let base = pattern.strip_suffix("/**").unwrap_or(pattern);
    if base.starts_with('/') {
        normalized == base || normalized.starts_with(&format!("{base}/"))
    } else {
        let suffix = format!("/{base}");
        normalized == base
            || normalized.ends_with(&suffix)
            || normalized.contains(&format!("{suffix}/"))
    }
}

use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;
use std::path::Path;

pub(super) fn check(registry: &Value, root: &Path, out: &mut Vec<Failure>) {
    let state = &registry["lease_state"];
    let rows = state
        .get("active_records")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let status = str_field(state, "status");
    if (status == "unissued" && !rows.is_empty()) || (status == "active" && rows.is_empty()) {
        out.push(Failure::new(
            "authority-lease",
            "lease_state_records_mismatch",
            "lease_state",
        ));
    }
    if rows.len() > 4 {
        out.push(Failure::new(
            "authority-lease",
            "lease_concurrency_limit",
            rows.len().to_string(),
        ));
    }
    let p0_count = rows
        .iter()
        .filter(|row| str_field(row, "exception_id") == "P0-DEBT-REPAIR")
        .count();
    if p0_count > 0 && (p0_count != 1 || rows.len() != 1) {
        out.push(Failure::new(
            "authority-lease",
            "p0_lease_must_be_serial_root_only",
            "active_records",
        ));
    }
    for row in &rows {
        record::validate_record(row, registry, root, out);
    }
    for (index, left) in rows.iter().enumerate() {
        for right in rows.iter().skip(index + 1) {
            overlap::compare_records(
                left,
                right,
                registry["lease_state"]
                    .get("worktree_root")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                out,
            );
        }
    }
    let Some(template) = state.get("record_template") else {
        out.push(Failure::new(
            "authority-lease",
            "lease_template_missing",
            "record_template",
        ));
        return;
    };
    record::validate_record(template, registry, root, out);
}
