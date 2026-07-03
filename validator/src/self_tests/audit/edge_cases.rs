use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn archive_writer_and_archive_builder_cover_production_zip_path() {
    let root = crate::self_tests::boundaries::support::temp_root("archive-edge");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    std::fs::write(root.join("docs/file.txt"), "archive payload").expect("payload");
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        serde_json::to_vec(&json!({"resources":["docs/file.txt"]})).expect("manifest"),
    )
    .expect("manifest");

    let mut rows = vec![crate::archive::zip::Entry {
        name: "root/file.txt".to_string(),
        bytes: b"payload".to_vec(),
        crc32: crate::archive::zip::crc32(b"payload"),
        offset: 0,
    }];
    let direct_zip = root.join("direct.zip");
    crate::archive::zip::write_zip(&direct_zip, &mut rows).expect("write zip");
    let bytes = crate::digest::read_file_bytes(&direct_zip).expect("zip bytes");
    assert!(bytes.starts_with(&0x04034b50u32.to_le_bytes()));
    assert!(
        bytes
            .windows(4)
            .any(|chunk| chunk == 0x02014b50u32.to_le_bytes())
    );
    assert!(rows[0].offset == 0);

    let receipt = crate::archive::build_archive(
        &root,
        &root.join("candidate.zip"),
        "candidate-root",
        "candidate_review_anchor",
    )
    .expect("archive receipt");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["archive"]["entry_count"], 1);
    assert_eq!(
        receipt["claim_ceiling"],
        "detached candidate review anchor only; not upload or distribution proof"
    );
    std::fs::remove_dir_all(root).expect("cleanup archive edge");
}

#[test]
fn audit_artifact_schema_edges_cover_enum_and_red_fallbacks() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let check_ids = crate::audit::artifacts::check_ids(&store);
    assert!(check_ids.iter().any(|id| id == "schema-valid"));
    assert!(
        check_ids
            .iter()
            .any(|id| id == "cli-performance-latency-speed-iteration-fitness")
    );

    let temp = crate::self_tests::boundaries::support::temp_root("artifact-fallback");
    std::fs::create_dir_all(&temp).expect("temp");
    assert_eq!(
        crate::audit::artifacts::safe_red_ids(&temp),
        vec!["red-catalog-unavailable".to_string()]
    );
    std::fs::remove_dir_all(temp).expect("cleanup artifact fallback");
}

#[test]
fn coverage_scope_package_success_path_reads_manifest_and_scripts() {
    let root = crate::self_tests::boundaries::support::temp_root("coverage-scope-success");
    for dir in ["templates/.harness", "templates/scripts"] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for (rel, text) in [
        (
            "templates/.harness/coverage-command",
            "cargo llvm-cov --workspace\n",
        ),
        (
            "templates/scripts/check-coverage-fast",
            "#!/usr/bin/env bash\nset -euo pipefail\ncargo test --offline\n",
        ),
        (
            "templates/scripts/check-coverage-full",
            "#!/usr/bin/env bash\nset -euo pipefail\ncargo llvm-cov --workspace --fail-under-lines 100\n",
        ),
        ("templates/COVERAGE_RECEIPT.json", "{}\n"),
    ] {
        std::fs::write(root.join(rel), text).expect("coverage surface");
    }
    let manifest = json!({
        "schema": "harness-ultragoal.coverage-manifest.v1",
        "coverage_command_path": ".harness/coverage-command",
        "repo_root_digest": crate::digest::bytes(b"coverage-root"),
        "required_target_paths": ["validator/src/lib.rs"],
        "repo_owned_source_roots": ["validator"],
        "changed_file_coupling_policy": {
            "required": true,
            "changed_files_digest": crate::digest::bytes(b"changed")
        },
        "required_measured_dimensions_per_root": [
            {"root":"validator","dimensions":["line","branch","function","artifact","ui_state"]}
        ],
        "repo_walk_policy": {
            "classify_all_nonignored_files": true,
            "ignored_local_state_cannot_be_target": true
        },
        "policy_mutation_gate": {"material_review_required": true},
        "receipt_freshness_binding": {
            "source_digest_required": true,
            "command_digest_required": true,
            "tool_generated": true,
            "prose_percent_forbidden": true
        },
        "tool_generated_proof_policy": {"receipt_required": true},
        "fast_full_gate_split": {
            "full_required_for_completion": true,
            "fast_supports_completion": false
        },
        "behavior_dimension_mapping": {
            "product_surface_claims": ["ui_state"],
            "generated_artifacts": ["artifact"]
        }
    });
    crate::json_boundary::write_json(
        &root.join("templates/.harness/coverage-manifest.json"),
        &manifest,
    )
    .expect("manifest");
    let failures = crate::audit::coverage::scope::package_failures(&root);
    assert!(
        failures
            .iter()
            .all(|failure| !failure.starts_with("coverage_manifest_malformed")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup coverage scope success");
}

#[test]
fn plugin_self_law_line_scan_recurses_and_reports_over_cap_source() {
    let root = crate::self_tests::boundaries::support::temp_root("self-law-lines");
    for dir in [
        "validator/src/nested",
        ".harness/nested",
        "scripts",
        ".codex-plugin",
        "validation_artifacts/coverage",
    ] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    std::fs::write(
        root.join("plugin-manifest-draft.json"),
        json!({"version":"0.0.11"}).to_string(),
    )
    .expect("manifest");
    std::fs::write(
        root.join(".codex-plugin/plugin.json"),
        json!({"version":"0.0.11"}).to_string(),
    )
    .expect("plugin");
    std::fs::write(root.join("validator/src/nested/deep.rs"), "fn ok() {}\n").expect("deep");
    std::fs::write(root.join(".harness/nested/run.sh"), "#!/usr/bin/env bash\n").expect("script");
    std::fs::write(root.join("scripts/check"), "#!/usr/bin/env bash\n").expect("check");
    std::fs::write(root.join(".harness/coverage-command"), "coverage\n").expect("coverage command");
    std::fs::write(
        root.join("validator/src/too_large.rs"),
        "fn oversized() {}\n".repeat(251),
    )
    .expect("large");
    std::fs::write(
        root.join("validation_artifacts/coverage/coverage-receipt.json"),
        json!({
            "coverage":{"policy":"100_percent_required","percent":100.0},
            "uncovered_records":[],
            "claim_ceiling":"supports_complete_coverage_claim",
            "supported_claim_classes":["complete_coverage"],
            "blocked_claim_classes":[
                "completion",
                "package_readiness",
                "review_readiness",
                "release_readiness",
                "final_packet_correctness",
                "update_goal_eligibility",
                "app_registry_or_reviewer_exposure"
            ],
            "target_revision":{"value":crate::self_tests::boundaries::support::sha('a')}
        })
        .to_string(),
    )
    .expect("coverage");
    let store = crate::schema_catalog::load(&crate::self_tests::boundaries::support::repo_root());
    let failures = crate::audit::plugin::laws::package_failures(&root, &store);
    assert!(
        failures.iter().any(|failure| failure
            .contains("plugin_self_law_line_cap_exceeded:validator/src/too_large.rs:251")),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup self law lines");
}

#[test]
fn red_fixture_observation_uses_package_and_nonfirst_semantic_routes() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let package = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &BTreeMap::new(),
        &json!({}),
        &json!({"check_id":"source-card-freshness","error":"source_card_catalog_missing"}),
        &json!({"schema":"bad"}),
        "docs/source-cards.json",
    );
    assert_eq!(package.check, "source-card-freshness");

    let mut bad =
        crate::json_boundary::read_json(&root.join("fixtures/valid/minimal-goal-run.json"))
            .expect("valid semantic fixture");
    bad["completion_manifest"]["claims"][0]["evidence"][0]["surface"] = json!("runtime_cli");
    let failures = crate::claim_semantics::semantic_failures(&bad, &root, &BTreeMap::new());
    let target = failures
        .iter()
        .find(|failure| failure.check_id == "claim-evidence-coupling")
        .expect("claim evidence coupling failure");
    let observed = crate::red::fixture::observation::observe_materialized(
        &root,
        &store,
        &BTreeMap::new(),
        &json!({
            "json_patch":[{
                "op":"replace",
                "path":"/completion_manifest/claims/0/evidence/0/surface",
                "value":"runtime_cli"
            }],
            "materialization":{"first_failure_must_match_expected":false}
        }),
        &json!({"check_id":target.check_id,"error":target.error}),
        &bad,
        "fixtures/valid/semantic-claim-boundary.json",
    );
    assert!(observed.ok, "{} {}", observed.check, observed.error);
}
