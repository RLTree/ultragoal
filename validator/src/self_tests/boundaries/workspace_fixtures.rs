use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator has repo parent")
        .to_path_buf()
}

pub(crate) fn temp_root(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let index = COUNTER.fetch_add(1, Ordering::Relaxed);
    repo_root()
        .join("target")
        .join(format!("ultragoal-boundary-{label}-{stamp}-{index}"))
}

pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path.parent().expect("test JSON path has a parent");
    std::fs::create_dir_all(parent).map_err(|err| {
        format!(
            "{}: test JSON parent create failed: {err}",
            parent.display()
        )
    })?;
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("{}: test JSON encoding failed: {err}", path.display()))?;
    std::fs::write(path, format!("{text}\n"))
        .map_err(|err| format!("{}: test JSON write failed: {err}", path.display()))
}

pub(crate) fn sha(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

#[test]
fn schema_receipt_rules_cover_required_and_kind_boundaries() {
    let root = repo_root();
    let store = crate::schema_catalog::load(&root);
    let missing = json!({});
    let errors = crate::schema_catalog::schema_errors(
        &store,
        "semantic-classification-receipt.schema.json",
        &missing,
    );
    assert!(
        errors
            .iter()
            .any(|err| err.contains("claim_id is required"))
    );
    let base = json!({
        "schema": "harness-ultragoal.semantic-classification-receipt.v1",
        "claim_id": "CLAIM-001",
        "canonical_text_digest": sha('a'),
        "classifier_contract_id": "contract",
        "classifier_contract_version": "v1",
        "classifier_implementation_kind": "deterministic_backstop",
        "generated_at": "2026-06-25T00:00:00Z",
        "producer_actor_id": "producer",
        "classifier_actor_id": "classifier",
        "actor_disjoint": true,
        "detected_semantic_classes": ["runtime_cli_backend_only_engine_only"],
        "rationale": "typed boundary test",
        "confidence": 0.99,
        "ambiguity": false,
        "required_proof_gates": ["ready_for_merge_receipt"],
        "claim_ceiling_recommendation": "withhold",
        "receipt_digest": sha('b')
    });
    assert!(
        crate::schema_catalog::schema_errors(
            &store,
            "semantic-classification-receipt.schema.json",
            &base
        )
        .is_empty()
    );

    let mut model = base.clone();
    model["classifier_implementation_kind"] = json!("model");
    let model_errors = crate::schema_catalog::schema_errors(
        &store,
        "semantic-classification-receipt.schema.json",
        &model,
    );
    assert!(
        model_errors
            .iter()
            .any(|err| err.contains("model receipts require provider_model"))
    );

    let mut human = base.clone();
    human["classifier_implementation_kind"] = json!("human_reviewer");
    let human_errors = crate::schema_catalog::schema_errors(
        &store,
        "semantic-classification-receipt.schema.json",
        &human,
    );
    assert!(
        human_errors
            .iter()
            .any(|err| err.contains("human reviewer receipts require classifier_evidence"))
    );

    let mut deterministic = base;
    deterministic["provider_model"] = json!({"provider": "p", "model": "m"});
    let deterministic_errors = crate::schema_catalog::schema_errors(
        &store,
        "semantic-classification-receipt.schema.json",
        &deterministic,
    );
    assert!(
        deterministic_errors
            .iter()
            .any(|err| err.contains("deterministic receipts must not carry external proof"))
    );
}

#[test]
fn audit_boundary_edges_reject_malformed_policy_surfaces() {
    let exclusions = json!({
        "exclusions": [
            {"path": "validator/src/lib.rs", "counts_as_covered": true},
            {"path": "external/generated.rs", "reviewed": false, "counts_as_covered": false}
        ]
    });
    let failures = crate::audit::coverage::scope::exclusions::failures(&exclusions);
    assert!(failures.contains(&"coverage_exclusion_missing_rationale".to_string()));
    assert!(failures.contains(&"coverage_exclusion_unreviewed".to_string()));
    assert!(failures.contains(&"coverage_exclusion_counted_as_covered".to_string()));
    assert!(failures.contains(&"coverage_repo_owned_code_excluded".to_string()));

    let root = temp_root("review-history");
    std::fs::create_dir_all(root.join("docs")).expect("docs");
    let mut failures = BTreeMap::new();
    crate::audit::review_history::check(&root, &mut failures);
    assert!(failures["validator-execution-provenance"][0].contains("missing"));
    std::fs::write(
        root.join("docs/review-loop-record.md"),
        "## Round 35 Current Sign-Off\ncurrent internal four-reviewer sign-off\n",
    )
    .expect("review record");
    failures.clear();
    crate::audit::review_history::check(&root, &mut failures);
    assert_eq!(failures["validator-execution-provenance"].len(), 2);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn coverage_scope_policy_reports_missing_surfaces_and_weakened_authority() {
    let root = temp_root("coverage-scope");
    std::fs::create_dir_all(&root).expect("coverage root");
    let missing = crate::audit::coverage::scope::package_failures(&root);
    assert!(
        missing
            .iter()
            .any(|item| item.starts_with("coverage_scope_surface_missing"))
    );
    assert!(
        missing
            .iter()
            .any(|item| item.starts_with("coverage_manifest_malformed"))
    );

    let bad = json!({
        "schema": "wrong",
        "coverage_command_path": "scripts/coverage",
        "repo_root_digest": crate::digest::ZERO,
        "required_target_paths": ["target"],
        "repo_owned_source_roots": ["src"],
        "changed_file_coupling_policy": {
            "required": false,
            "changed_files_digest": "not-a-digest"
        },
        "required_measured_dimensions_per_root": [{
            "root": "src",
            "dimensions": ["line"]
        }],
        "repo_walk_policy": {
            "classify_all_nonignored_files": false,
            "ignored_local_state_cannot_be_target": false
        },
        "policy_mutation_gate": {"material_review_required": false},
        "receipt_freshness_binding": {},
        "tool_generated_proof_policy": {},
        "fast_full_gate_split": {
            "full_required_for_completion": false,
            "fast_supports_completion": true
        },
        "behavior_dimension_mapping": {}
    });
    let failures = crate::audit::coverage::scope::value_failures_with_root(&bad, None);
    for expected in [
        "coverage_receipt_malformed:schema",
        "coverage_command_missing:path",
        "coverage_receipt_source_digest_mismatch",
        "coverage_manifest_target_gap",
        "coverage_source_missing_from_manifest",
        "coverage_changed_file_claim_not_withheld",
        "coverage_receipt_changed_files_digest_mismatch",
        "coverage_source_unclassified",
        "coverage_target_path_ignored_local_state",
        "coverage_policy_weakened_without_review",
        "coverage_receipt_manifest_digest_mismatch",
        "coverage_receipt_command_digest_mismatch",
        "coverage_receipt_not_tool_generated",
        "coverage_percent_from_prose",
        "coverage_full_gate_missing",
        "coverage_fast_gate_used_for_completion",
        "coverage_ui_state_missing_for_product_surface",
        "coverage_artifact_dimension_missing_for_generated_authority",
    ] {
        assert!(failures.contains(&expected.to_string()), "{expected}");
    }
    std::fs::remove_dir_all(root).expect("cleanup coverage scope");
}
