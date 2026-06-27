use serde_json::json;

#[test]
fn completion_manifest_accepts_canonical_product_law_surfaces() {
    let root = crate::self_tests::boundaries::support::repo_root();
    let store = crate::schema_catalog::load(&root);
    let manifest = json!({
        "schema":"harness-ultragoal.completion-manifest.v1",
        "contract_bundle_hash":crate::digest::ZERO,
        "required_claim_ids":["CLAIM-001"],
        "required_claim_ids_hash":crate::digest::ZERO,
        "goal_binding":{
            "schema":"harness-ultragoal.goal-binding.v1",
            "goal_id":"GOAL-001",
            "contract_digest":crate::digest::ZERO,
            "contract_path":"docs/contract.md",
            "bound_at":"2026-06-26T00:00:00Z"
        },
        "commit":"abcdef0",
        "root":"fixture-root",
        "claims":[{
            "id":"CLAIM-001",
            "title":"Product Cohesion claim",
            "claim_scope":"required",
            "claim_kind":"product::cohesion",
            "status":"proven_live",
            "claim_surface":"product::cohesion",
            "allowed_evidence_surfaces":["product::cohesion","product::fitness","ui_browser"],
            "evidence":[{
                "id":"EV-001",
                "claim_id":"CLAIM-001",
                "kind":"product_cohesion_receipt",
                "surface":"product::cohesion",
                "path":"validation_artifacts/product-cohesion/journey-receipt.json",
                "digest":crate::digest::ZERO,
                "produced_by_command_id":"cmd-001",
                "commit":"abcdef0",
                "workspace":"/repo",
                "observed_at":"2026-06-26T00:00:00Z",
                "freshness":{
                    "policy":"target head and evidence commit must match",
                    "checked_at":"2026-06-26T00:00:00Z",
                    "current_commit":"abcdef0",
                    "evidence_commit":"abcdef0",
                    "verdict":"current"
                },
                "product_cohesion_task":{
                    "primary_journey_id":"intent-to-proof",
                    "ui_evidence_paths":["validation_artifacts/product-cohesion/run-detail.png"],
                    "expected_interruption_rate":"rare",
                    "exhausted_harness_evidence_paths":["validation_artifacts/product-cohesion/harness-paths.json"]
                }
            },{
                "id":"EV-002",
                "claim_id":"CLAIM-001",
                "kind":"product::fitness::receipt",
                "surface":"product::fitness",
                "path":"validation_artifacts/harness/product-fitness-receipt.json",
                "digest":crate::digest::ZERO,
                "produced_by_command_id":"cmd-001",
                "commit":"abcdef0",
                "workspace":"/repo",
                "observed_at":"2026-06-26T00:00:00Z",
                "freshness":{
                    "policy":"target head and evidence commit must match",
                    "checked_at":"2026-06-26T00:00:00Z",
                    "current_commit":"abcdef0",
                    "evidence_commit":"abcdef0",
                    "verdict":"current"
                }
            }],
            "claim_ceiling_effect":"included"
        }],
        "verification_backlog_path":"validation_artifacts/backlog.json",
        "verification_backlog_digest":crate::digest::ZERO,
        "validator_receipts":[],
        "generated_at":"2026-06-26T00:00:00Z"
    });
    let errors =
        crate::schema_catalog::schema_errors(&store, "completion-manifest.schema.json", &manifest);
    assert!(
        !errors.iter().any(|err| err.contains("claim_kind")
            || err.contains("claim_surface")
            || err.contains("surface")),
        "{errors:?}"
    );
}
