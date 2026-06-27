use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const PATH: &str = "docs/law-family-aliases.json";
const SCHEMA: &str = "harness-ultragoal.law-family-aliases.v1";
const AUTHORITY: &str = "source_obligation_and_mandatory_law_ids";
const ALIAS_ROLE: &str = "explanatory_family_alias_only";
const CLAIM_AUTHORITY: &str = "none";

const REQUIRED_VOCABULARY: &[&str] = &[
    "claim-governance system",
    "no claim exists until CLI-bound typed same-surface evidence exists",
    "maximally factored module tree",
    "no residual prefix encoding",
    "governed namespace class",
    "validator theater",
    "row-shape compliance",
    "exception-shaped loophole",
    "stale receipt",
    "same-surface proof",
    "lowering the claim ceiling is not compliance",
];

const TIGHTENING_FIELDS: &[&str] = &[
    "governed_classes_are_not_waivers",
    "namespace_law_has_zero_exceptions",
    "every_path_resolves_to_exactly_one_governed_class",
    "bootstrap_receipts_are_transition_only",
    "coverage_requires_exact_100_percent_and_empty_uncovered_records",
    "product_fitness_ownership_disposition_first_class",
    "same_law_id_enforcement_required_across_all_surfaces",
];

pub(crate) fn failures(root: &Path) -> Vec<String> {
    let value = match crate::json_boundary::read_json(&root.join(PATH)) {
        Ok(value) => value,
        Err(err) => return vec![format!("{PATH}: {err}")],
    };
    let source_ids = ids(
        root,
        "docs/source-obligation-matrix.json",
        "obligations",
        "id",
    );
    let mandatory_ids = ids(root, "docs/mandatory-law-surfaces.json", "laws", "law_id");
    value_failures(&value, &source_ids, &mandatory_ids)
}

pub(crate) fn value_failures(
    value: &Value,
    source_ids: &BTreeSet<String>,
    mandatory_ids: &BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        out.push("law_family_aliases_wrong_schema".to_string());
    }
    if value.get("canonical_authority").and_then(Value::as_str) != Some(AUTHORITY) {
        out.push("law_family_aliases_wrong_canonical_authority".to_string());
    }
    if value
        .get("canonical_id_replacement_allowed")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push("law_family_aliases_replacement_allowed".to_string());
    }
    if value
        .get("aliases_claim_authority")
        .and_then(Value::as_bool)
        != Some(false)
    {
        out.push("law_family_aliases_claim_authority_allowed".to_string());
    }
    let declared_count = value
        .get("canonical_law_count")
        .and_then(Value::as_u64)
        .unwrap_or_default() as usize;
    if declared_count != source_ids.len() || declared_count != mandatory_ids.len() {
        out.push(format!(
            "law_family_aliases_canonical_count_mismatch:{declared_count}:source={}:mandatory={}",
            source_ids.len(),
            mandatory_ids.len()
        ));
    }
    out.extend(vocabulary_failures(value));
    out.extend(tightening_failures(value));
    out.extend(alias_failures(value, source_ids, mandatory_ids));
    out
}

fn vocabulary_failures(value: &Value) -> Vec<String> {
    let terms = value
        .get("doctrine_vocabulary")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    REQUIRED_VOCABULARY
        .iter()
        .filter(|term| !terms.contains(**term))
        .map(|term| format!("law_family_aliases_missing_vocabulary:{term}"))
        .collect()
}

fn tightening_failures(value: &Value) -> Vec<String> {
    let Some(tightening) = value.get("tightening") else {
        return vec!["law_family_aliases_missing_tightening".to_string()];
    };
    TIGHTENING_FIELDS
        .iter()
        .filter(|field| tightening.get(**field).and_then(Value::as_bool) != Some(true))
        .map(|field| format!("law_family_aliases_tightening_not_true:{field}"))
        .collect()
}

fn alias_failures(
    value: &Value,
    source_ids: &BTreeSet<String>,
    mandatory_ids: &BTreeSet<String>,
) -> Vec<String> {
    let Some(rows) = value.get("aliases").and_then(Value::as_array) else {
        return vec!["law_family_aliases_missing_aliases".to_string()];
    };
    let mut out = Vec::new();
    if rows.len() != 24 {
        out.push(format!("law_family_aliases_wrong_count:{}", rows.len()));
    }
    let expected_hu_ids = expected_hu_ids();
    let mut seen_hu_ids = BTreeSet::new();
    let mut covered_canonical = BTreeSet::new();
    for row in rows {
        let hu_id = row.get("hu_id").and_then(Value::as_str).unwrap_or("");
        if !expected_hu_ids.contains(hu_id) {
            out.push(format!("law_family_aliases_unknown_hu_id:{hu_id}"));
        }
        if !seen_hu_ids.insert(hu_id.to_string()) {
            out.push(format!("law_family_aliases_duplicate_hu_id:{hu_id}"));
        }
        if row.get("authority_role").and_then(Value::as_str) != Some(ALIAS_ROLE) {
            out.push(format!("law_family_aliases_wrong_role:{hu_id}"));
        }
        if row.get("claim_authority").and_then(Value::as_str) != Some(CLAIM_AUTHORITY) {
            out.push(format!("law_family_aliases_claim_authority:{hu_id}"));
        }
        if row
            .get("replaces_canonical_law_ids")
            .and_then(Value::as_bool)
            != Some(false)
        {
            out.push(format!("law_family_aliases_replaces_canonical:{hu_id}"));
        }
        out.extend(canonical_mapping_failures(
            hu_id,
            row,
            source_ids,
            mandatory_ids,
            &mut covered_canonical,
        ));
    }
    for hu_id in expected_hu_ids {
        if !seen_hu_ids.contains(&hu_id) {
            out.push(format!("law_family_aliases_missing_hu_id:{hu_id}"));
        }
    }
    for canonical in source_ids {
        if !covered_canonical.contains(canonical) {
            out.push(format!(
                "law_family_aliases_unmapped_canonical_id:{canonical}"
            ));
        }
    }
    out
}

fn canonical_mapping_failures(
    hu_id: &str,
    row: &Value,
    source_ids: &BTreeSet<String>,
    mandatory_ids: &BTreeSet<String>,
    covered_canonical: &mut BTreeSet<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    let Some(ids) = row.get("canonical_law_ids").and_then(Value::as_array) else {
        return vec![format!("law_family_aliases_missing_canonical_ids:{hu_id}")];
    };
    if ids.is_empty() {
        out.push(format!("law_family_aliases_empty_canonical_ids:{hu_id}"));
    }
    for value in ids {
        let Some(id) = value.as_str() else {
            out.push(format!(
                "law_family_aliases_non_string_canonical_id:{hu_id}"
            ));
            continue;
        };
        if id.starts_with("HU-") {
            out.push(format!(
                "law_family_aliases_hu_used_as_canonical:{hu_id}:{id}"
            ));
        }
        if !source_ids.contains(id) {
            out.push(format!("law_family_aliases_unknown_source_id:{hu_id}:{id}"));
        }
        if !mandatory_ids.contains(id) {
            out.push(format!(
                "law_family_aliases_unknown_mandatory_id:{hu_id}:{id}"
            ));
        }
        covered_canonical.insert(id.to_string());
    }
    out
}

fn expected_hu_ids() -> BTreeSet<String> {
    (1..=24).map(|index| format!("HU-{index:03}")).collect()
}

fn ids(root: &Path, path: &str, array_key: &str, id_key: &str) -> BTreeSet<String> {
    crate::json_boundary::read_json(&root.join(path))
        .ok()
        .and_then(|value| value.get(array_key).and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get(id_key).and_then(Value::as_str).map(str::to_string))
        .collect()
}
