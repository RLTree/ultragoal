use serde_json::{Value, json};
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("copy target");
    for entry in std::fs::read_dir(from).expect("read fixture dir") {
        let entry = entry.expect("dir entry");
        let dest = to.join(entry.file_name());
        if entry.file_type().expect("file type").is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).expect("copy fixture file");
        }
    }
}

#[test]
fn branch_arms_close_last_gaps() {
    let root =
        crate::self_tests::boundaries::support::temp_root("cli_performance_observability-lanes");
    let digest_map = crate::audit::artifacts::digest_map(&[
        json!({}),
        json!({"path": 5, "digest": 7}),
        json!({"path": "missing-digest"}),
        json!({"path": "bad-digest", "digest": 7}),
        json!({"path": "ok", "digest": "sha256:abc"}),
    ]);
    assert_eq!(digest_map.get("ok").map(String::as_str), Some("sha256:abc"));
    assert!(
        crate::audit::artifacts::validator_artifact_rel(Path::new("/root"), Path::new("/other"))
            .expect_err("outside validator artifact rejected")
            .contains("validator artifact root strip failed")
    );
    assert!(
        crate::audit::artifacts::artifact_ref(&root, "../escape.json")
            .expect_err("artifact path invalid")
            .contains("escapes package root")
    );

    let long = "x".repeat(90);
    let medium = "x".repeat(40);
    let lanes = json!({"lanes": [
        {"id":"short-independent","lane_size_evidence":{
            "owned_path_groups":["a","b","c"],"work_units":["a","b","c","d"],
            "independent_outcome":"","why_not_parent_inline":medium,"why_not_smaller":medium}},
        {"id":"short-parent","lane_size_evidence":{
            "owned_path_groups":["a","b","c"],"work_units":["a","b","c","d"],
            "independent_outcome":long,"why_not_parent_inline":"","why_not_smaller":medium}},
        {"id":"short-smaller","lane_size_evidence":{
            "owned_path_groups":["a","b","c"],"work_units":["a","b","c","d"],
            "independent_outcome":long,"why_not_parent_inline":medium,"why_not_smaller":""}}
    ]});
    let mut failures = Vec::new();
    crate::claim_semantics::lane::policy::check_lanes(
        &json!({}),
        &lanes,
        &json!({}),
        &[json!({"lane_id": "ready-lane"})],
        &root,
        0,
        &mut failures,
    );
    assert_eq!(
        failures
            .iter()
            .filter(|failure| failure.error == "lane_too_small_or_overlapping")
            .count(),
        3
    );

    let perf =
        crate::self_tests::boundaries::support::temp_root("cli_performance_observability-cli-perf");
    write_json(&perf.join("plugin-manifest-draft.json"), &json!({}));
    let command = crate::cli::performance::PerformanceCommand {
        operation: crate::cli::performance::types::PerformanceOperation::Prove,
        receipt: None,
        class: crate::cli::performance::types::BudgetClass::Focused,
    };
    let missing_perf = crate::self_tests::boundaries::support::temp_root(
        "cli_performance_observability-cli-run-missing",
    );
    let err = crate::cli::performance::run(&missing_perf, &command)
        .expect_err("run propagates receipt build errors");
    assert!(err.contains("plugin-manifest-draft"), "{err}");
    assert_eq!(
        crate::cli::performance::proof::cli_binary_digest_for_path(None),
        crate::digest::ZERO
    );
    assert_eq!(
        crate::cli::performance::proof::cli_binary_digest_for_path(Some(perf.join("missing-bin"))),
        crate::digest::ZERO
    );
    assert_eq!(
        crate::cli::performance::proof::digest_or_zero(&perf, "missing-config.json"),
        crate::digest::ZERO
    );
    let err = crate::cli::performance::receipt(&perf, &command, 1)
        .expect_err("missing schema catalog digest is fail-closed");
    assert!(err.contains("schema-catalog.json"), "{err}");
    for (rel, next_missing) in [
        ("schemas/schema-catalog.json", "mandatory-law-surfaces.json"),
        (
            "docs/mandatory-law-surfaces.json",
            "templates/agent-standards/enforcement.json",
        ),
        (
            "templates/agent-standards/enforcement.json",
            "templates/RED_FIXTURES.json",
        ),
        (
            "templates/RED_FIXTURES.json",
            "source-obligation-matrix.json",
        ),
        ("docs/source-obligation-matrix.json", ""),
    ] {
        write_json(&perf.join(rel), &json!({}));
        match crate::cli::performance::receipt(&perf, &command, 1) {
            Ok(value) => assert!(next_missing.is_empty(), "{value}"),
            Err(err) => {
                assert!(!next_missing.is_empty(), "{err}");
                assert!(err.contains(next_missing), "{err}");
            }
        }
    }

    let schemas =
        crate::self_tests::boundaries::support::temp_root("cli_performance_observability-schemas");
    write_json(
        &schemas.join("schemas/schema-catalog.json"),
        &json!({"schemas": [
            {"id":"target.schema.json","path":"schemas/target.schema.json"},
            {"id":"external-success.schema.json","path":"schemas/external-success.schema.json"},
            {"id":"external-missing.schema.json","path":"schemas/external-missing.schema.json"}
        ]}),
    );
    write_json(
        &schemas.join("schemas/target.schema.json"),
        &json!({"$id":"target.schema.json","$defs":{"text":{"type":"string"}}}),
    );
    write_json(
        &schemas.join("schemas/external-success.schema.json"),
        &json!({"$id":"external-success.schema.json","$ref":"schemas/target.schema.json#/$defs/text"}),
    );
    write_json(
        &schemas.join("schemas/external-missing.schema.json"),
        &json!({"$id":"external-missing.schema.json","$ref":"schemas/target.schema.json#/missing"}),
    );
    let store = crate::schema_catalog::load(&schemas);
    assert!(
        !crate::schema_catalog::schema_errors(&store, "external-success.schema.json", &json!("ok"))
            .iter()
            .any(|err| err.contains("unresolved schema ref"))
    );
    assert!(
        crate::schema_catalog::schema_errors(&store, "external-missing.schema.json", &json!("ok"))
            .iter()
            .any(|err| err.contains("unresolved schema ref"))
    );

    let semantic =
        crate::self_tests::boundaries::support::temp_root("cli_performance_observability-semantic");
    write_json(
        &semantic.join("claims.json"),
        &json!({"claims":[{"id":"CLAIM-OK_value","title":"CLI claim"}]}),
    );
    crate::semantic::receipt::generate(crate::semantic::receipt::GenerateOptions {
        root: semantic.clone(),
        input: "claims.json".into(),
        out_dir: semantic.join("out"),
        implementation_kind: "deterministic_backstop".to_string(),
        provider: None,
        model: None,
        contract_id: "contract".to_string(),
        contract_version: "v1".to_string(),
        prompt_contract_digest: None,
        producer_actor_id: "producer".to_string(),
        classifier_actor_id: "classifier".to_string(),
    })
    .expect("semantic receipt");
    assert!(
        semantic
            .join("out/CLAIM-OK_value.semantic-classification-receipt.json")
            .is_file()
    );

    let repo = crate::self_tests::boundaries::support::repo_root();
    let target_repo = crate::self_tests::boundaries::support::temp_root(
        "cli_performance_observability-observe-sh",
    );
    copy_dir(
        &repo.join("fixtures/target-repo/valid-observability"),
        &target_repo,
    );
    std::fs::rename(
        target_repo.join("scripts/observe"),
        target_repo.join("scripts/observe.sh"),
    )
    .expect("move observe fallback");
    let (receipt, _) = crate::target_repo::audit_target_repo(
        &target_repo,
        "fresh-init",
        "observe sh fallback",
        &[],
        true,
        false,
        None,
    );
    assert_eq!(receipt["checks"]["observability-stack"]["status"], "pass");
    let script = target_repo.join("scripts/check");
    let text = std::fs::read_to_string(&script).expect("check script");
    std::fs::write(
        &script,
        text.replace("echo \"harness-check:observability pass\"\n", ""),
    )
    .expect("remove observability marker");
    let (blocked_receipt, _) = crate::target_repo::audit_target_repo(
        &target_repo,
        "fresh-init",
        "observe marker required",
        &[],
        true,
        false,
        None,
    );
    assert!(
        blocked_receipt["checks"]["observability-stack"]["detail"]
            .as_str()
            .unwrap_or("")
            .contains("gate marker")
    );

    let _ = std::fs::remove_dir_all(root);
    let _ = std::fs::remove_dir_all(perf);
    let _ = std::fs::remove_dir_all(missing_perf);
    let _ = std::fs::remove_dir_all(schemas);
    let _ = std::fs::remove_dir_all(semantic);
    let _ = std::fs::remove_dir_all(target_repo);
}
