use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn errors(out: &[crate::audit::contract::Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn source_obligation_toml_and_symlink_edges() {
    let root =
        crate::self_tests::boundaries::support::temp_root("source_obligation_fallthrough-edges");

    let obligation_failures = crate::audit::source_obligations::value_failures(&json!({
        "obligations": [{
            "id": "fresh-non-special-law",
            "obligation": "deterministic validator red fixture receipt proof",
            "package_surface": "source install cache review packet",
            "enforcement_disposition": "deterministic validator red fixture receipt",
            "missing_validation_fixture_or_receipt": "receipt present",
            "claim_ceiling_impact": "blocks related completion claims",
            "coverage_status": "deterministic validator coverage"
        }]
    }));
    assert!(
        !obligation_failures
            .iter()
            .any(|failure| failure.starts_with("fresh-non-special-law:")),
        "{obligation_failures:?}"
    );

    std::fs::create_dir_all(root.join("custom-agents")).expect("agents");
    std::fs::write(
        root.join("custom-agents/harness-product-simplicity-falsifier.toml"),
        r#"
name = "Wrong Visible Name"
description = 42
developer_instructions = "not enough"
"#,
    )
    .expect("agent toml");
    let mut plugin = Vec::new();
    crate::claim_semantics::plugin_policy::check_plugin(
        &json!({
            "plugin_manifest": {
                "skills": [],
                "agents": [{
                    "name": "harness-product-simplicity-falsifier",
                    "path": "custom-agents/harness-product-simplicity-falsifier.toml",
                    "app_visible_name": "Harness Product Simplicity Falsifier"
                }],
                "resources": []
            }
        }),
        &root,
        &mut plugin,
    );
    let got = errors(&plugin);
    assert!(got.contains(&"custom_agent_name_mismatch"), "{plugin:?}");
    assert!(
        got.contains(&"custom_agent_toml_field_missing"),
        "{plugin:?}"
    );

    let repo = crate::self_tests::boundaries::support::repo_root();
    let mut matrix =
        crate::json_boundary::read_json(&repo.join("docs/source-obligation-matrix.json"))
            .expect("source obligation matrix");
    let topology = matrix["obligations"]
        .as_array_mut()
        .expect("obligations")
        .iter_mut()
        .find(|row| {
            row.get("id").and_then(Value::as_str) == Some("validator-source-namespace-topology")
        })
        .expect("validator topology row");
    topology["missing_validation_fixture_or_receipt"] = json!("typed red proof present");
    topology["claim_ceiling_impact"] = json!("blocks validator source topology claims");
    let topology_failures = crate::audit::source_obligations::value_failures(&matrix);
    assert!(
        topology_failures
            .iter()
            .any(|failure| failure.contains("source_obligation_missing_tamper")),
        "{topology_failures:?}"
    );

    write_json(
        &root.join("validation_artifacts/product-cohesion/symlink-fixture.json"),
        &json!({
            "schema": "harness-ultragoal.target-fixture-symlink.v1",
            "link_path": "edge-link",
            "target_path": "target.txt"
        }),
    );
    let fixture = crate::target_fixtures::materialize_symlink_fixture(&root)
        .expect("symlink materialized")
        .expect("fixture present");
    assert_eq!(
        fixture.link.file_name().and_then(|name| name.to_str()),
        Some("edge-link")
    );
    fixture.cleanup().expect("cleanup symlink");

    std::fs::remove_dir_all(root).expect("cleanup source_obligation_fallthrough edges");
}
