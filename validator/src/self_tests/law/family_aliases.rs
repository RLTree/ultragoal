use serde_json::{Value, json};
use std::collections::BTreeSet;

fn has(items: &[String], needle: &str) -> bool {
    items.iter().any(|item| item.contains(needle))
}

fn live_value() -> Value {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    crate::json_boundary::read_json(&root.join("docs/law-family-aliases.json"))
        .expect("law family aliases")
}

fn live_ids(path: &str, array_key: &str, id_key: &str) -> BTreeSet<String> {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    crate::json_boundary::read_json(&root.join(path))
        .expect("registry")
        .get(array_key)
        .and_then(Value::as_array)
        .expect("rows")
        .iter()
        .map(|row| {
            row.get(id_key)
                .and_then(Value::as_str)
                .expect("id")
                .to_string()
        })
        .collect()
}

fn failures(value: &Value) -> Vec<String> {
    crate::audit::law::family::aliases::value_failures(
        value,
        &live_ids("docs/source-obligation-matrix.json", "obligations", "id"),
        &live_ids("docs/mandatory-law-surfaces.json", "laws", "law_id"),
    )
}

#[test]
fn hu_family_aliases_cover_canonical_laws_without_replacing_them() {
    let mut value = live_value();
    assert_eq!(failures(&value), Vec::<String>::new());

    value["aliases"][0]["canonical_law_ids"][0] = json!("HU-001");
    let errors = failures(&value);
    assert!(has(&errors, "law_family_aliases_hu_used_as_canonical"));
    assert!(has(&errors, "law_family_aliases_unknown_source_id"));
    assert!(has(&errors, "law_family_aliases_unknown_mandatory_id"));
}

#[test]
fn hu_family_aliases_fail_if_given_claim_or_replacement_authority() {
    let mut value = live_value();
    value["canonical_id_replacement_allowed"] = json!(true);
    value["aliases_claim_authority"] = json!(true);
    value["aliases"][0]["authority_role"] = json!("canonical_law_id");
    value["aliases"][0]["claim_authority"] = json!("completion");
    value["aliases"][0]["replaces_canonical_law_ids"] = json!(true);

    let errors = failures(&value);
    for expected in [
        "law_family_aliases_replacement_allowed",
        "law_family_aliases_claim_authority_allowed",
        "law_family_aliases_wrong_role:HU-001",
        "law_family_aliases_claim_authority:HU-001",
        "law_family_aliases_replaces_canonical:HU-001",
    ] {
        assert!(
            errors.contains(&expected.to_string()),
            "{expected}: {errors:?}"
        );
    }
}

#[test]
fn hu_family_aliases_require_doctrine_vocabulary_and_tightening() {
    let mut value = live_value();
    value["doctrine_vocabulary"] = json!(["claim-governance system"]);
    value["tightening"]["namespace_law_has_zero_exceptions"] = json!(false);
    value["tightening"]["same_law_id_enforcement_required_across_all_surfaces"] = json!(null);

    let errors = failures(&value);
    assert!(has(
        &errors,
        "law_family_aliases_missing_vocabulary:same-surface proof"
    ));
    assert!(has(
        &errors,
        "law_family_aliases_tightening_not_true:namespace_law_has_zero_exceptions"
    ));
    assert!(has(
        &errors,
        "law_family_aliases_tightening_not_true:same_law_id_enforcement_required_across_all_surfaces"
    ));
}

#[test]
fn hu_family_aliases_fail_unknown_and_unmapped_canonical_laws() {
    let mut value = live_value();
    value["aliases"][0]["canonical_law_ids"]
        .as_array_mut()
        .expect("canonical ids")
        .push(json!("not-a-canonical-law"));
    let errors = failures(&value);
    assert!(has(&errors, "law_family_aliases_unknown_source_id"));
    assert!(has(&errors, "law_family_aliases_unknown_mandatory_id"));

    let mut unmapped = live_value();
    let missing = live_ids("docs/source-obligation-matrix.json", "obligations", "id")
        .into_iter()
        .next()
        .expect("source id");
    for row in unmapped["aliases"].as_array_mut().expect("aliases") {
        row["canonical_law_ids"]
            .as_array_mut()
            .expect("canonical ids")
            .retain(|id| id.as_str() != Some(&missing));
    }
    let errors = failures(&unmapped);
    assert!(errors.contains(&format!(
        "law_family_aliases_unmapped_canonical_id:{missing}"
    )));
}

#[test]
fn hu_family_aliases_package_entrypoint_and_shape_edges_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
    assert_eq!(
        crate::audit::law::family::aliases::failures(&root),
        Vec::<String>::new()
    );

    let mut bad_header = live_value();
    bad_header["schema"] = json!("wrong");
    bad_header["canonical_authority"] = json!("hu_aliases");
    bad_header["canonical_law_count"] = json!(1);
    let errors = failures(&bad_header);
    assert!(has(&errors, "law_family_aliases_wrong_schema"));
    assert!(has(&errors, "law_family_aliases_wrong_canonical_authority"));
    assert!(has(&errors, "law_family_aliases_canonical_count_mismatch"));

    let mut missing_tightening = live_value();
    missing_tightening
        .as_object_mut()
        .expect("object")
        .remove("tightening");
    assert_eq!(
        failures(&missing_tightening),
        vec!["law_family_aliases_missing_tightening".to_string()]
    );

    let mut missing_aliases = live_value();
    missing_aliases
        .as_object_mut()
        .expect("object")
        .remove("aliases");
    assert_eq!(
        failures(&missing_aliases),
        vec!["law_family_aliases_missing_aliases".to_string()]
    );

    let mut wrong_count = live_value();
    wrong_count["aliases"]
        .as_array_mut()
        .expect("aliases")
        .pop();
    let errors = failures(&wrong_count);
    assert!(has(&errors, "law_family_aliases_wrong_count"));
    assert!(has(&errors, "law_family_aliases_missing_hu_id"));

    let mut unknown_duplicate = live_value();
    unknown_duplicate["aliases"][0]["hu_id"] = json!("HU-999");
    unknown_duplicate["aliases"][1]["hu_id"] = json!("HU-999");
    let errors = failures(&unknown_duplicate);
    assert!(has(&errors, "law_family_aliases_unknown_hu_id:HU-999"));
    assert!(has(&errors, "law_family_aliases_duplicate_hu_id:HU-999"));

    let mut missing_ids = live_value();
    missing_ids["aliases"][0]
        .as_object_mut()
        .expect("alias object")
        .remove("canonical_law_ids");
    assert!(has(
        &failures(&missing_ids),
        "law_family_aliases_missing_canonical_ids"
    ));

    let mut empty_ids = live_value();
    empty_ids["aliases"][0]["canonical_law_ids"] = json!([]);
    assert!(has(
        &failures(&empty_ids),
        "law_family_aliases_empty_canonical_ids"
    ));

    let mut non_string = live_value();
    non_string["aliases"][0]["canonical_law_ids"] = json!([7]);
    assert!(has(
        &failures(&non_string),
        "law_family_aliases_non_string_canonical_id"
    ));
}
