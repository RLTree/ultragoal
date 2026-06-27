use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn schema_keywords_reject_unresolved_refs_recursion_and_type_mismatch() {
    let root = crate::self_tests::boundaries::support::temp_root("schema-keyword-branches");
    write_json(
        &root.join("schemas/missing-ref.schema.json"),
        &json!({"$id":"missing-ref.schema.json","$ref":"absent.schema.json"}),
    );
    write_json(
        &root.join("schemas/recursive.schema.json"),
        &json!({
            "$id":"recursive.schema.json",
            "$ref":"#/definitions/node",
            "definitions":{"node":{"$ref":"#/definitions/node"}}
        }),
    );
    write_json(
        &root.join("schemas/object.schema.json"),
        &json!({"$id":"object.schema.json","type":"object"}),
    );
    write_json(
        &root.join("schemas/supported-shapes.schema.json"),
        &json!({"$id":"supported-shapes.schema.json","$defs":[],"allOf":{}}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[
            {"id":"missing-ref.schema.json","path":"schemas/missing-ref.schema.json"},
            {"id":"recursive.schema.json","path":"schemas/recursive.schema.json"},
            {"id":"object.schema.json","path":"schemas/object.schema.json"},
            {"id":"supported-shapes.schema.json","path":"schemas/supported-shapes.schema.json"}
        ]}),
    );
    let store = crate::schema_catalog::load(&root);
    let unresolved =
        crate::schema_catalog::schema_errors(&store, "missing-ref.schema.json", &json!({}));
    assert!(
        unresolved
            .iter()
            .any(|err| err.contains("unresolved schema ref absent.schema.json")),
        "{unresolved:?}"
    );
    let recursive =
        crate::schema_catalog::schema_errors(&store, "recursive.schema.json", &json!({}));
    assert!(
        recursive
            .iter()
            .any(|err| err.contains("schema ref depth exceeded")),
        "{recursive:?}"
    );
    let type_errors =
        crate::schema_catalog::schema_errors(&store, "object.schema.json", &json!("nope"));
    assert!(
        type_errors.iter().any(|err| err.contains("type mismatch")),
        "{type_errors:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup schema keyword branches");
}

#[test]
fn schema_keywords_cover_scalar_object_branch_and_contains_boundaries() {
    let root = crate::self_tests::boundaries::support::temp_root("schema-keyword-boundaries");
    write_json(
        &root.join("schemas/keywords.schema.json"),
        &json!({
            "$id":"keywords.schema.json",
            "type":"object",
            "minProperties":2,
            "maxProperties":2,
            "propertyNames":{"pattern":"^[a-z0-9_]+$"},
            "required":["name","timestamp","age"],
            "properties":{
                "name":{"type":"string","minLength":3,"pattern":"^[a-z0-9_]+$"},
                "timestamp":{"type":"string","format":"date-time"},
                "age":{"type":"number","minimum":10,"maximum":20},
                "choice":{"anyOf":[{"const":"A"},{"const":"B"}]},
                "one":{"oneOf":[{"type":"string"},{"type":"number"}]},
                "not_null":{"not":{"type":"null"}},
                "items":{
                    "type":"array",
                    "contains":{
                        "type":"object",
                        "required":["kind"],
                        "properties":{"kind":{"const":"x"}}
                    },
                    "minContains":2,
                    "maxContains":2
                }
            },
            "if":{"properties":{"mode":{"const":"strict"}},"required":["mode"]},
            "then":{"required":["strict_evidence"]},
            "else":{"not":{"required":["strict_evidence"]}}
        }),
    );
    write_json(
        &root.join("schemas/patterns.schema.json"),
        &json!({
            "$id":"patterns.schema.json",
            "type":"object",
            "required":["path"],
            "properties":{
                "path":{"type":"string","pattern":"(^examples/generated/|READY_FOR_MERGE|VALIDATOR_RECEIPT)"}
            }
        }),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[
            {"id":"keywords.schema.json","path":"schemas/keywords.schema.json"},
            {"id":"patterns.schema.json","path":"schemas/patterns.schema.json"}
        ]}),
    );
    let store = crate::schema_catalog::load(&root);
    let empty_errors =
        crate::schema_catalog::schema_errors(&store, "keywords.schema.json", &json!({}));
    assert!(empty_errors.iter().any(|err| err.contains("minProperties")));
    assert!(
        empty_errors
            .iter()
            .any(|err| err.contains("$.name: required"))
    );

    let scalar_errors = crate::schema_catalog::schema_errors(
        &store,
        "keywords.schema.json",
        &json!({
            "Bad-Name": true,
            "name": "X",
            "timestamp": "bad-time",
            "age": 5,
            "choice": "C",
            "one": true,
            "not_null": null,
            "items": [{"kind":"x"}],
            "mode": "strict"
        }),
    );
    for expected in [
        "maxProperties",
        "pattern mismatch",
        "minLength",
        "format date-time",
        "minimum",
        "anyOf mismatch",
        "oneOf mismatch",
        "not matched forbidden schema",
        "contains mismatch",
        "$.strict_evidence: required",
    ] {
        assert!(
            scalar_errors.iter().any(|err| err.contains(expected)),
            "{expected}: {scalar_errors:?}"
        );
    }

    let max_contains = crate::schema_catalog::schema_errors(
        &store,
        "keywords.schema.json",
        &json!({
            "name":"valid_name",
            "timestamp":"2026-06-25T00:00:00Z",
            "age":25,
            "items":[{"kind":"x"},{"kind":"x"},{"kind":"x"}],
            "strict_evidence": true
        }),
    );
    assert!(max_contains.iter().any(|err| err.contains("maximum")));
    assert!(max_contains.iter().any(|err| err.contains("maxContains")));
    assert!(
        max_contains
            .iter()
            .any(|err| err.contains("not matched forbidden schema"))
    );
    assert!(
        crate::schema_catalog::schema_errors(
            &store,
            "patterns.schema.json",
            &json!({"path":"VALIDATOR_RECEIPT-anchor.json"})
        )
        .is_empty()
    );
    std::fs::remove_dir_all(root).expect("cleanup schema keyword boundaries");
}
