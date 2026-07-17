use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn red_registry_and_semantic_boundaries() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "target_boundary-red-registry-semantic",
    );
    let mut registry_errors = Vec::new();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({
            "live_registry_exposure": {
                "path": "validation_artifacts/missing-exposure.json",
                "digest": crate::digest::ZERO
            }
        }),
        &mut registry_errors,
    );
    assert_eq!(
        registry_errors[0].error,
        "review_round_live_registry_artifact_malformed"
    );

    let opts = crate::semantic::receipt::GenerateOptions {
        root: root.clone(),
        input: "missing-claims.json".into(),
        out_dir: root.join("out"),
        implementation_kind: "deterministic_backstop".to_string(),
        provider: None,
        model: None,
        contract_id: "contract".to_string(),
        contract_version: "v1".to_string(),
        prompt_contract_digest: None,
        producer_actor_id: "producer".to_string(),
        classifier_actor_id: "classifier".to_string(),
    };
    let err = crate::semantic::receipt::generate(opts).expect_err("missing input rejected");
    assert!(
        err.contains("open failed") || err.contains("metadata failed"),
        "{err}"
    );
    std::fs::remove_dir_all(root).expect("cleanup red registry semantic");
}

#[test]
fn audit_schema_cli_edges() {
    let schemas =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("target_boundary-schemas");
    write_json(
        &schemas.join("schemas/schema-catalog.json"),
        &json!({
            "schemas": [
                {"id": "no-id.schema.json", "path": "schemas/no-id.schema.json"},
                {"id": "ref-missing.schema.json", "path": "schemas/ref-missing.schema.json"},
                {"id": "one-success.schema.json", "path": "schemas/one-success.schema.json"}
            ]
        }),
    );
    write_json(
        &schemas.join("schemas/no-id.schema.json"),
        &json!({"$defs": {"text": {"type": "string"}}, "$ref": "#/$defs/text"}),
    );
    write_json(
        &schemas.join("schemas/ref-missing.schema.json"),
        &json!({"$id": "ref-missing.schema.json", "$ref": "#/$defs/missing"}),
    );
    write_json(
        &schemas.join("schemas/one-success.schema.json"),
        &json!({"$id": "one-success.schema.json", "oneOf": [{"type": "string"}, {"type": "number"}]}),
    );
    let store = crate::schema_catalog::load(&schemas);
    let fallback_ids = crate::audit::artifacts::check_ids(&store);
    assert_eq!(
        fallback_ids[0],
        "active-setup-to-idle-orchestration-thread-bound-heartbeat"
    );
    let no_id_errors =
        crate::schema_catalog::schema_errors(&store, "no-id.schema.json", &json!("ok"));
    assert!(
        !no_id_errors
            .iter()
            .any(|err| err.contains("type mismatch") || err.contains("unresolved schema ref")),
        "{no_id_errors:?}"
    );
    let no_id_cached =
        crate::schema_catalog::schema_errors(&store, "no-id.schema.json", &json!("ok"));
    assert!(
        !no_id_cached
            .iter()
            .any(|err| err.contains("type mismatch") || err.contains("unresolved schema ref")),
        "{no_id_cached:?}"
    );
    assert!(
        crate::schema_catalog::schema_errors(&store, "ref-missing.schema.json", &json!("x"))
            .iter()
            .any(|err| err.contains("unresolved schema ref"))
    );
    let one_success =
        crate::schema_catalog::schema_errors(&store, "one-success.schema.json", &json!("x"));
    assert!(
        !one_success.iter().any(|err| err.contains("oneOf mismatch")),
        "{one_success:?}"
    );

    std::fs::remove_dir_all(schemas).expect("cleanup schemas");
}
