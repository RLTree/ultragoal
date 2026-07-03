use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn aggregate_errors_at(root: &Path, bundle: Value) -> Vec<String> {
    crate::claim_semantics::semantic_failures(&bundle, root, &BTreeMap::new())
        .into_iter()
        .map(|failure| failure.error)
        .collect()
}

#[test]
fn aggregate_semantic_failures_cover_contract_claim_goal_and_amendments() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("aggregate-semantic");
    std::fs::create_dir_all(&root).expect("aggregate root");
    let errors = aggregate_errors_at(
        &root,
        json!({
            "completion_manifest": {
                "root": ".",
                "contract_bundle_hash": crate::self_tests::boundaries::workspace_fixtures::sha('1'),
                "required_claim_ids": ["CLAIM-MISSING", "CLAIM-OPTIONAL"],
                "claims": [{
                    "id": "CLAIM-OPTIONAL",
                    "title": "Optional claim cannot close a required id",
                    "description": "This deliberately stays outside required scope.",
                    "status": "withheld",
                    "claim_scope": "optional",
                    "claim_ceiling_effect": "withheld_or_blocked",
                    "evidence": []
                },{
                    "id": "CLAIM-GOAL",
                    "title": "Goal-bound completion claim",
                    "description": "This included claim requires the active goal binding.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_ceiling_effect": "included",
                    "requires_goal_binding": true,
                    "evidence": []
                },{
                    "id": "CLAIM-OBS",
                    "title": "Observability claim",
                    "description": "This included observability claim has no receipt.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_ceiling_effect": "included",
                    "requires_observability": true,
                    "evidence": []
                },{
                    "id": "CLAIM-FEATURE",
                    "title": "Feature completion claim",
                    "description": "This included feature completion has no live E2E proof.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_kind": "feature_completion",
                    "claim_ceiling_effect": "included",
                    "evidence": []
                },{
                    "id": "CLAIM-CONNECTOR",
                    "title": "Connector capability claim",
                    "description": "This included connector capability has no discovery receipt.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_kind": "connector_capability",
                    "claim_ceiling_effect": "included",
                    "evidence": []
                }],
                "goal_binding": {
                    "status": "unavailable",
                    "goal_id": "goal-1",
                    "objective": "current objective",
                    "contract_path": "GOAL_CONTRACT.md",
                    "discovery_probe_receipt": {"verdict": "available"},
                    "get_goal_receipt": {
                        "workspace": "other-workspace",
                        "observed_goal_id": "goal-2",
                        "observed_objective": "old objective",
                        "observed_contract_path": "old-contract.md",
                        "observed_status": "blocked"
                    }
                }
            },
            "lane_registry": {"lanes": [], "root_verification_stages": []},
            "ready_for_merge": {
                "lane_id": "LANE-X",
                "ready": false,
                "contract_bundle_digest": crate::self_tests::boundaries::workspace_fixtures::sha('2'),
                "changed_files": ["validator/src/unmeasured.rs"]
            },
            "ready_for_merge_receipts": [],
            "verification_backlog": {"rows": []},
            "plugin_manifest": {},
            "plugin_flow": {},
            "automation_tick_receipt": {
                "freshness_policy": {"clock_at_validation": "bad-clock"}
            },
            "amendments": [
                {
                    "amendment_id": "AMEND-1",
                    "change_class": "clarifies",
                    "removed_or_weakened_claim_ids": ["CLAIM-OLD"]
                },
                {
                    "amendment_id": "AMEND-2",
                    "change_class": "weakens",
                    "approval": {"artifact": {"path": "missing-approval.json"}}
                }
            ]
        }),
    );

    for expected in [
        "required_claim_missing",
        "required_claim_not_required_scope",
        "contract_bundle_hash_mismatch",
        "goal_tools_unavailable_without_probe",
        "goal_binding_receipt_stale_or_mismatched",
        "goal_bound_claim_without_bound_goal",
        "observability_claim_without_receipt",
        "live_beneficial_e2e_missing",
        "connector_capability_without_discovery",
        "coverage_changed_file_without_receipt",
        "weakening_mislabeled_as_clarification",
        "weakening_without_explicit_approval",
    ] {
        assert!(
            errors.contains(&expected.to_string()),
            "{expected}: {errors:?}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup aggregate semantic");
}

#[test]
fn aggregate_semantic_failures_cover_text_surface_overclaims() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("aggregate-text-overclaims");
    std::fs::create_dir_all(&root).expect("aggregate text root");
    let errors = aggregate_errors_at(
        &root,
        json!({
            "completion_manifest": {
                "root": ".",
                "contract_bundle_hash": crate::self_tests::boundaries::workspace_fixtures::sha('0'),
                "contract_bundle_hash_actual": crate::self_tests::boundaries::workspace_fixtures::sha('0'),
                "required_claim_ids": [],
                "claims": [{
                    "id": "",
                    "title": "Empty claim id is ignored by duplicate-id collection",
                    "status": "withheld",
                    "claim_scope": "optional",
                    "claim_ceiling_effect": "withheld_or_blocked",
                    "evidence": []
                },{
                    "id": "CLAIM-INSTALL-BUTTON",
                    "title": "Install button works",
                    "description": "The install button succeeded without a promotion receipt.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_ceiling_effect": "included",
                    "evidence": []
                },{
                    "id": "CLAIM-INSTALLATION",
                    "title": "Installation success",
                    "description": "Installation works without external surface evidence.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_ceiling_effect": "included",
                    "evidence": []
                },{
                    "id": "CLAIM-LIBRARY",
                    "title": "Team library plugin listed",
                    "description": "The plugin is discoverable in the team library.",
                    "status": "pass",
                    "claim_scope": "required",
                    "claim_ceiling_effect": "included",
                    "evidence": []
                },{
                    "id": "CLAIM-BACKEND-APP",
                    "title": "Backend app is visible",
                    "description": "The backend app is visible in a headless server log.",
                    "status": "withheld",
                    "claim_scope": "required",
                    "claim_ceiling_effect": "withheld_or_blocked",
                    "evidence": []
                }]
            },
            "lane_registry": {"lanes": [], "root_verification_stages": []},
            "ready_for_merge": {"changed_files": []},
            "ready_for_merge_receipts": [],
            "verification_backlog": {"rows": []},
            "plugin_manifest": {},
            "plugin_flow": {}
        }),
    );
    let count = errors
        .iter()
        .filter(|error| *error == "claim_text_requires_unproven_surface")
        .count();
    assert_eq!(count, 3, "{errors:?}");
    std::fs::remove_dir_all(root).expect("cleanup aggregate text overclaims");
}
