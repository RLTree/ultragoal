use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn package_failures_for_current_law(
    root: &Path,
    law: &str,
    current_failures: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let registry = match crate::json_boundary::read_json(&root.join(super::REGISTRY)) {
        Ok(value) => value,
        Err(error) => return vec![format!("{}: {error}", super::REGISTRY)],
    };
    let mut failures = super::value_failures(root, &registry);
    let rows = registry
        .get("laws")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.get("law_id").and_then(Value::as_str) == Some(law))
        .collect::<Vec<_>>();
    if rows.is_empty() {
        failures.push(format!("mandatory_law_current_row_missing:{law}"));
        return stable(failures);
    }
    if rows.len() != 1 {
        failures.push(format!("mandatory_law_current_row_duplicate:{law}"));
        return stable(failures);
    }
    let current_digest = match crate::package::inventory::package_digest(root) {
        Ok(digest) => digest,
        Err(error) => {
            failures.push(format!(
                "mandatory_law_current_candidate_digest_unavailable:{law}:{error}"
            ));
            String::new()
        }
    };
    let row = rows[0];
    failures.extend(super::receipt_value_failures_with_candidate(
        root,
        row,
        &current_digest,
    ));
    let store = crate::schema_catalog::load(root);
    failures.extend(super::dependencies::anti_theater_failures(
        root, &store, law,
    ));
    failures.extend(super::production::current_check_failures(
        row,
        law,
        current_failures,
    ));
    stable(failures)
}

fn stable(mut failures: Vec<String>) -> Vec<String> {
    failures.sort();
    failures.dedup();
    failures
}
