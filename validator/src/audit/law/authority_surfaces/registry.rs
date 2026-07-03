use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const LAW_REQUIREMENTS: &[LawRequirement] = &[
    LawRequirement {
        law: "authority-source-binding",
        check: "authority-source-binding",
        guards: &[
            "parser_boundary_projection_or_catalog_classification",
            "typed_downstream_records",
            "stale_digest_rejected",
            "source_obligation_parity",
        ],
    },
    LawRequirement {
        law: "typed-records-over-prose",
        check: "typed-records-over-prose",
        guards: &[
            "raw_downstream_json_authority_rejected",
            "raw_path_authority_rejected",
            "raw_string_authority_rejected",
            "raw_map_authority_rejected",
            "source_obligation_parity",
        ],
    },
    LawRequirement {
        law: "generated-proof-artifact-provenance-anti-fabrication",
        check: "generated-proof-artifact-provenance-anti-fabrication",
        guards: &[
            "builder_contract_excluded_from_package_evidence",
            "runtime_normalized_fixture_not_artifact_truth",
            "generated_rows_carry_provenance",
            "manually_edited_generated_report",
            "stale_output_digest",
        ],
    },
    LawRequirement {
        law: "distinct-proof-surfaces-claim-ceilings",
        check: "distinct-proof-surfaces-claim-ceilings",
        guards: &[
            "source_local_proof_substituted_for_live_surface",
            "tests_substituted_for_product_behavior",
            "receipt_existence_substituted_for_behavior",
            "wrong_proof_surface",
            "source_obligation_parity",
        ],
    },
    LawRequirement {
        law: "total-authority-types-impossible-state-elimination",
        check: "total-authority-types-impossible-state-elimination",
        guards: &[
            "absolute_receipt_write_without_output_authority",
            "private_path_claim_output",
            "broad_exception_without_owner",
            "generic_fallback_overclaim",
            "session_history_label_without_owner",
        ],
    },
    LawRequirement {
        law: "validator-theater-miswire-resistance",
        check: "validator-theater-miswire-resistance",
        guards: &[
            "generic_output_only",
            "row_shape_only_mechanization",
            "fallback_overclaim",
        ],
    },
];

struct LawRequirement {
    law: &'static str,
    check: &'static str,
    guards: &'static [&'static str],
}

pub(super) fn law_registry_failures(
    root: &Path,
    registry: &Value,
    red_ids: &BTreeSet<String>,
    inventory: &BTreeSet<String>,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for requirement in LAW_REQUIREMENTS {
        let Some(row) = law_row(registry, requirement.law) else {
            push(
                &mut out,
                requirement.check,
                format!("authority_surface_law_missing:{}", requirement.law),
            );
            continue;
        };
        if row.get("validator_check_id").and_then(Value::as_str) != Some(requirement.check) {
            push(
                &mut out,
                requirement.check,
                format!(
                    "authority_surface_wrong_validator_check:{}",
                    requirement.law
                ),
            );
        }
        require_inventory_path(
            root,
            inventory,
            row,
            "valid_fixture_path",
            requirement,
            &mut out,
        );
        require_guard_fixtures(root, inventory, row, red_ids, requirement, &mut out);
    }
    out
}

fn require_guard_fixtures(
    root: &Path,
    inventory: &BTreeSet<String>,
    row: &Value,
    red_ids: &BTreeSet<String>,
    requirement: &LawRequirement,
    out: &mut Vec<(String, String)>,
) {
    for guard in requirement.guards {
        if row
            .pointer(&format!("/law_specific/{guard}"))
            .and_then(Value::as_bool)
            != Some(true)
        {
            push(
                out,
                requirement.check,
                format!(
                    "authority_surface_guard_missing:{}:{guard}",
                    requirement.law
                ),
            );
        }
        let red_id = format!("{}-{}-red", requirement.law, guard.replace('_', "-"));
        let rel = format!("fixtures/red/{red_id}.json");
        if !red_ids.contains(&red_id) {
            push(
                out,
                requirement.check,
                format!("authority_surface_red_fixture_missing:{red_id}"),
            );
        } else if !root.join(&rel).is_file() || !inventory.contains(&rel) {
            push(
                out,
                requirement.check,
                format!("authority_surface_red_fixture_not_packaged:{red_id}"),
            );
        }
    }
}

fn require_inventory_path(
    root: &Path,
    inventory: &BTreeSet<String>,
    row: &Value,
    key: &str,
    requirement: &LawRequirement,
    out: &mut Vec<(String, String)>,
) {
    let Some(rel) = row.get(key).and_then(Value::as_str) else {
        push(
            out,
            requirement.check,
            format!("authority_surface_field_missing:{}:{key}", requirement.law),
        );
        return;
    };
    if !root.join(rel).is_file() || !inventory.contains(rel) {
        push(
            out,
            requirement.check,
            format!(
                "authority_surface_path_not_packaged:{}:{rel}",
                requirement.law
            ),
        );
    }
}

fn law_row<'a>(registry: &'a Value, law: &str) -> Option<&'a Value> {
    registry
        .get("laws")
        .and_then(Value::as_array)?
        .iter()
        .find(|row| row.get("law_id").and_then(Value::as_str) == Some(law))
}

pub(super) fn red_catalog_ids(value: &Value) -> BTreeSet<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

fn push(out: &mut Vec<(String, String)>, check: &str, detail: String) {
    out.push((check.to_string(), detail));
}
