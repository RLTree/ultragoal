use crate::review::round::ReviewFailure;
use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}

fn errors(out: &[ReviewFailure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

#[test]
fn schema_catalog_reports_catalog_rows_refs_and_missing_schema() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("schema-catalog");
    let missing = crate::schema_catalog::load(&root);
    assert!(
        missing
            .errors
            .iter()
            .any(|err| err.contains("schema-catalog load failed"))
    );

    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":{}}),
    );
    let bad_catalog = crate::schema_catalog::load(&root);
    assert!(
        bad_catalog
            .errors
            .contains(&"schema-catalog schemas must be a list".to_string())
    );

    write_json(
        &root.join("schemas/mismatch.schema.json"),
        &json!({"$id":"other"}),
    );
    write_json(
        &root.join("schemas/good.schema.json"),
        &json!({"$id":"good","$ref":"missing"}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[
            {},
            {"id":"escape","path":"../escape.json"},
            {"id":"missing","path":"schemas/missing.schema.json"},
            {"id":"mismatch","path":"schemas/mismatch.schema.json"},
            {"id":"good","path":"schemas/good.schema.json"}
        ]}),
    );
    let store = crate::schema_catalog::load(&root);
    let joined = store.errors.join("\n");
    for expected in [
        "schema row invalid",
        "escape: package path escapes package root",
        "missing: schema load failed",
        "mismatch: schema id mismatch",
        "good:",
    ] {
        assert!(joined.contains(expected), "{expected}: {joined}");
    }
    assert_eq!(
        crate::schema_catalog::schema_errors(&store, "absent.schema.json", &json!({})),
        vec!["offline_schema_resolution_failed: missing schema absent.schema.json".to_string()]
    );
    std::fs::remove_dir_all(root).expect("cleanup schema catalog");
}

#[test]
fn namespace_law_reports_paths_exceptions_bindings_and_orphans() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-law");
    write_text(&root.join("orphan.txt"), "orphan");
    let manifest = json!({
        "resources": [
            "utils/tool.rs",
            "common/domain.rs",
            "shared/state.rs",
            "lib/core.rs",
            "services/run.rs",
            "a/b/c/d/e/f/g/h/i.txt",
            "root-clutter-file.md",
            "domain/alpha-one.rs",
            "domain/alpha-two.rs",
            "domain/alpha-three.rs"
        ]
    });
    let failures = crate::audit::namespace::law::value_failures(&root, &manifest);
    for expected in [
        "namespace_junk_drawer_path",
        "namespace_vague_common_domain_path",
        "namespace_vague_shared_domain_path",
        "namespace_vague_lib_domain_path",
        "namespace_generic_services_path",
        "namespace_excessive_depth",
        "namespace_root_clutter_without_route",
        "namespace_orphan_repo_file",
    ] {
        assert!(
            failures.iter().any(|item| item.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
    let source_manifest = json!({
        "resources": [
            "validator/src/domain/schema_foo.rs",
            "validator/src/domain/schema_fee.rs"
        ]
    });
    let source_failures = crate::audit::namespace::law::value_failures(&root, &source_manifest);
    assert!(
        source_failures
            .iter()
            .any(|item| item.contains("namespace_validator_source_residual_prefix_encoding")),
        "{source_failures:?}"
    );
    let classes = json!({"schema":"harness-ultragoal.namespace-class-registry.v1","classes":[
        {"id":"","kind":"unknown","description":"","authority":"","claim_ceiling_impact":"raises","surface_globs":[],"maximal_factoring_required":false,"waiver_allowed":true,"authority_path":"missing.md"},
        {"id":"contract","kind":"external_compatibility","description":"compat","authority":"compat","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["schemas/**/*.json"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"missing.md"},
        {"id":"generated","kind":"fixture_catalog","description":"generated","authority":"fixtures","claim_ceiling_impact":"classifies_surface_without_raising_claim_ceiling","surface_globs":["validator/src/generated.rs"],"maximal_factoring_required":true,"waiver_allowed":false,"authority_path":"missing.json","exception_type":"repeated_prefix"}
    ]});
    let class_failures =
        crate::audit::namespace::law::class_registry_value_failures(&root, &classes);
    for expected in [
        "namespace_class_untyped",
        "namespace_class_kind_invalid",
        "namespace_class_authority_missing:contract",
        "namespace_class_authority_missing:generated",
        "namespace_class_waiver_field_present:generated:exception_type",
        "namespace_class_generated_for_hand_authored_source:generated",
    ] {
        assert!(class_failures.iter().any(|item| item.contains(expected)));
    }
    let package_failures = crate::audit::namespace::law::package_failures(&root, &manifest);
    assert!(
        package_failures
            .iter()
            .any(|item| item.contains("namespace_class_registry_file_missing"))
    );
    std::fs::remove_dir_all(root).expect("cleanup namespace law");
}

#[test]
fn namespace_law_reports_large_orphan_sets_as_counted_samples() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-law-many-orphans");
    for index in 0..21 {
        write_text(&root.join(format!("docs/orphan-{index}.txt")), "orphan");
    }
    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));
    assert!(failures.iter().any(|item| {
        item.starts_with("namespace_orphan_repo_file_count:21:sample:")
            && item.contains("docs/orphan-0.txt")
    }));
    std::fs::remove_dir_all(root).expect("cleanup many orphans");
}

#[test]
fn review_target_and_registry_exposure_fail_closed() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("review-registry");
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["docs/a.txt","validation_artifacts/harness/excluded.json"]}),
    );
    write_text(&root.join("docs/a.txt"), "a");
    write_json(
        &root.join("validation_artifacts/harness/excluded.json"),
        &json!({"proof":true}),
    );
    write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &json!({"status":"pass"}),
    );
    let receipt = crate::package::build_review_target_receipt(&root).expect("review target");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["excluded_path_count"], 1);
    assert!(receipt.get("validator_receipt").is_some());
    assert!(
        crate::package::review_payload(&root, "missing.txt")
            .expect_err("missing review payload")
            .contains("review target manifest path is missing")
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":["../escape.txt"]}),
    );
    assert!(
        crate::package::build_review_target_receipt(&root)
            .expect_err("invalid review target")
            .contains("review target is not closed")
    );

    let mut out = Vec::<ReviewFailure>::new();
    crate::review::round::registry::exposure_errors(&root, &json!({}), &mut out);
    assert!(errors(&out).contains(&"review_round_live_registry_exposure_missing"));
    out.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"live_registry_exposure":{"path":"../escape.json","digest":crate::digest::ZERO}}),
        &mut out,
    );
    assert!(errors(&out).contains(&"review_round_live_registry_artifact_invalid"));
    let exposure = root.join("exposure.json");
    std::fs::write(&exposure, "{").expect("malformed exposure");
    out.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"live_registry_exposure":{"path":"exposure.json","digest":crate::digest::file(&exposure).expect("digest")}}),
        &mut out,
    );
    assert!(errors(&out).contains(&"review_round_live_registry_artifact_malformed"));
    write_json(
        &exposure,
        &json!({"schema":"wrong","source":"disk","captured_at":"old","session_id":"","round_id":"other","agent_types":[]}),
    );
    out.clear();
    crate::review::round::registry::exposure_errors(
        &root,
        &json!({"generated_at":"now","round_id":"round","reviewers":[{"persona":"contract_claim_falsifier","live_spawn_receipt":{"source_thread_id":"wrong"}}],"live_registry_exposure":{"path":"exposure.json","digest":crate::digest::file(&exposure).expect("digest")}}),
        &mut out,
    );
    let got = errors(&out);
    for expected in [
        "review_round_live_registry_artifact_malformed",
        "review_round_live_registry_stale",
        "review_round_live_registry_agent_missing",
    ] {
        assert!(got.contains(&expected), "{expected}: {got:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup review registry");
}
