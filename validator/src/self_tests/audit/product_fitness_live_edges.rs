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
fn audit_catalog_and_receipt_edges() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "product_fitness_live-audit-edges",
    );
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    );
    write_json(
        &root.join("schemas/schema-catalog.json"),
        &json!({"schemas":[]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[]}),
    );
    write_json(
        &root.join("docs/source-obligation-matrix.json"),
        &json!({"obligations":[]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[]}),
    );
    write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]));

    let control = crate::audit::cli::control_plane::authority::package_failures(&root);
    for expected in [
        "cli_control_plane_package_inventory_missing:validator/src/cli/control/plane/mod.rs",
        "cli_control_plane_schema_catalog_missing_receipt_schema",
        "cli_control_plane_missing_standards_row:cli-control-plane-authority",
        "cli_control_plane_missing_source_obligation:cli-control-plane-authority",
        "cli_control_plane_missing_foundational_trace:cli-control-plane-authority",
        "cli_control_plane_missing_valid_fixture:cli-control-plane-authority",
        "cli_control_plane_missing_red_fixture:cli-control-plane-authority-checklist-completion-without-cli-red",
    ] {
        assert!(
            control.iter().any(|failure| failure == expected),
            "{expected}: {control:?}"
        );
    }
    let performance = crate::audit::cli::performance::package_failures(&root);
    assert!(
        performance
            .iter()
            .any(|failure| { failure == "cli_performance_schema_catalog_missing_receipt_schema" })
    );
    assert!(
        performance
            .iter()
            .any(|failure| { failure.starts_with("cli_performance_missing_red_fixture:") })
    );

    let standards = crate::audit::agent::standards::enforcement::value_failures(
        &json!({"rows":[{
            "id":"informational-law","source_law":"source","required_behavior":"behavior",
            "owner_lane":"lane","current_status":"current","blocker_or_repair_action":"none",
            "required_execplan_refs":["plan"],"enforcement_status":"informational",
            "claim_ids_affected":"completion"
        }]}),
        None,
    );
    assert!(standards.iter().any(|failure| {
        failure == "agent_standards_informational_overclaims:informational-law"
    }));

    let exclusions = crate::audit::coverage::scope::exclusions::failures(&json!({
        "exclusions":[{"path":"external/vendor.txt","counts_as_covered":true}]
    }));
    for expected in [
        "coverage_exclusion_missing_rationale",
        "coverage_exclusion_unreviewed",
        "coverage_exclusion_counted_as_covered",
    ] {
        assert!(
            exclusions.contains(&expected.to_string()),
            "{expected}: {exclusions:?}"
        );
    }
    assert_ne!(
        crate::audit::fit_repo_receipt::canonical_digest(&json!({})),
        crate::audit::fit_repo_receipt::canonical_digest(&json!("scalar"))
    );
    assert_ne!(
        crate::audit::product::fitness::receipt::canonical_digest(&json!({})),
        crate::audit::product::fitness::receipt::canonical_digest(&json!("scalar"))
    );
    let product = crate::audit::product::fitness::receipt::canonical_package_failures(
        &root,
        &json!({"claim":{"id":"WRONG"}}),
    );
    assert!(product.contains(&"product_fitness_receipt_wrong_claim_id".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup product_fitness_live audit edges");
}

#[test]
fn claim_and_materiality_edges() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "product_fitness_live-claim-edges",
    );
    let mut out = Vec::new();
    crate::claim_semantics::claim::evidence::live_e2e_check(
        &json!({"id":"CLAIM","evidence":[{
            "id":"live","kind":"live_beneficial_e2e","digest":crate::digest::ZERO,
            "path":"fixtures/mock-run.json",
            "live_beneficial_task":{
                "real_input_path":"fixture/input.txt","output_artifact_path":"mock/output.txt"
            }
        }]}),
        &json!({}),
        &root,
        &mut out,
    );
    assert!(errors(&out).contains(&"live_beneficial_e2e_not_live"));
    let receipt = json!({
        "coverage":{"policy":"ratchet_floor"},
        "measured_dimensions":[],
        "target_paths":["validator/src/lib.rs"]
    });
    crate::claim_semantics::coverage::receipt::rules::dimensions(
        &receipt,
        "source code branch error path",
        &mut out,
    );
    assert!(errors(&out).contains(&"coverage_required_dimension_missing"));

    let mut product = Vec::new();
    let receipt_path = root.join("receipts/product-fitness.json");
    write_json(
        &receipt_path,
        &json!({"claim":{"id":"OTHER","repeated_use_claimed":false}}),
    );
    crate::claim_semantics::product::fitness::check(
        &json!({
            "id":"PF","title":"daily driver repeated use claim",
            "description":"This is a daily driver with repeated use and retention.",
            "claim_ceiling_effect":"included","requires_product_cohesion":true,
            "evidence":[{
                "kind":"product::fitness::receipt","surface":"product::fitness",
                "path":"receipts/product-fitness.json",
                "digest":crate::digest::file(&receipt_path).expect("fitness digest")
            }]
        }),
        &root,
        &mut product,
    );
    assert!(errors(&product).contains(&"product_fitness_daily_driver_overclaim"));
    let blocked = crate::review::materiality::value_failures(&json!({
        "decision":"BLOCKED_BEFORE_REVIEW",
        "reviewers_required":["product"],
        "validator_repairs_recommended":[]
    }));
    assert!(blocked.contains(&"materiality_blocked_launches_reviewers".to_string()));
    assert!(blocked.contains(&"materiality_blocked_without_repair".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup product_fitness_live claim edges");
}
