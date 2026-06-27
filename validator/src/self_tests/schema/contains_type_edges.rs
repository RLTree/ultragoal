use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn schema_keywords_cover_type_arrays_and_contains_fast_path_fallbacks() {
    let root = crate::self_tests::boundaries::support::temp_root("schema-keyword-extra-boundaries");
    write_json(
        &root.join("schemas/extra.schema.json"),
        &json!({
            "$id":"extra.schema.json",
            "type":"object",
            "properties":{
                "unknown_type":{"type":["unknown","null"]},
                "non_schema_type":{"type":true},
                "string_contains":{
                    "type":"array",
                    "contains":{"type":"string"},
                    "minContains":1
                },
                "partial_const_contains":{
                    "type":"array",
                    "contains":{
                        "type":"object",
                        "required":["kind","mode"],
                        "properties":{"kind":{"const":"x"}}
                    },
                    "minContains":1
                },
                "object_const_contains":{
                    "type":"array",
                    "contains":{
                        "type":"object",
                        "required":["kind"],
                        "properties":{"kind":{"const":"x"}}
                    },
                    "minContains":1
                }
            }
        }),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[{"id":"extra.schema.json","path":"schemas/extra.schema.json"}]}),
    );
    let store = crate::schema_catalog::load(&root);
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "extra.schema.json",
        &json!({
            "unknown_type": 7,
            "non_schema_type": 7,
            "string_contains": [7],
            "partial_const_contains": [{"kind":"x"}],
            "object_const_contains": [7]
        }),
    );
    assert!(
        errors
            .iter()
            .any(|err| err.contains("$.string_contains: contains mismatch")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|err| err.contains("$.partial_const_contains: contains mismatch")),
        "{errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|err| err.contains("$.object_const_contains: contains mismatch")),
        "{errors:?}"
    );
    assert!(
        errors.iter().all(|err| !err.contains("unknown_type")),
        "unknown schema types must not create authority: {errors:?}"
    );
    assert!(
        errors.iter().all(|err| !err.contains("non_schema_type")),
        "non-schema type values must not create authority: {errors:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup schema keyword extra boundaries");
}
