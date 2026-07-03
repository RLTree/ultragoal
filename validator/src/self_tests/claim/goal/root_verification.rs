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
fn aggregate_semantic_failures_cover_goal_receipt_and_root_verification_edges() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("aggregate-root-verification");
    std::fs::create_dir_all(root.join("artifacts")).expect("artifacts");
    let post_receipt_path = root.join("artifacts/post-merge.json");
    std::fs::write(
        &post_receipt_path,
        serde_json::to_vec(&json!({
            "schema": "harness-ultragoal.root-verification-receipt.v1",
            "stage": "post_merge_integration_gate",
            "upstream_lane_id": "missing-lane",
            "upstream_commit": "head",
            "target_branch": "main",
            "validated_at": "2026-06-25T00:00:00Z",
            "merge_reachability": {"upstream_commit_reachable": true},
            "commands": [{"id": "post-cmd", "exit": 0}]
        }))
        .expect("post receipt json"),
    )
    .expect("post receipt");
    let post_digest = crate::digest::file(&post_receipt_path).expect("post digest");
    let root_str = root.to_string_lossy().to_string();
    let base = json!({
        "completion_manifest": {
            "root": root_str,
            "contract_bundle_hash": crate::self_tests::boundaries::workspace_fixtures::sha('0'),
            "contract_bundle_hash_actual": crate::self_tests::boundaries::workspace_fixtures::sha('0'),
            "required_claim_ids": [],
            "claims": [],
            "goal_binding": {
                "status": "available",
                "goal_id": "goal-1",
                "objective": "current objective",
                "contract_path": "GOAL_CONTRACT.md",
                "get_goal_receipt": {
                    "workspace": root_str,
                    "observed_goal_id": "goal-1",
                    "observed_objective": "current objective",
                    "observed_contract_path": "old-contract.md",
                    "observed_status": "active"
                }
            }
        },
        "lane_registry": {
            "lanes": [{
                "id": "lane-1",
                "status": "active",
                "workspace": "work/lane-1",
                "owned_paths": ["src"]
            }],
            "root_verification_stages": {
                "pre_merge_lane_gate": {"status": "pass"},
                "post_merge_integration_gate": {
                    "stage": "post_merge_integration_gate",
                    "status": "pass",
                    "validated_at": "2026-06-25T00:00:00Z",
                    "command_receipt": {
                        "id": "post-cmd",
                        "exit": 0,
                        "artifact_path": "artifacts/post-merge.json",
                        "artifact_digest": post_digest
                    },
                    "artifact_receipt": {
                        "path": "artifacts/post-merge.json",
                        "digest": post_digest
                    }
                },
                "final_all_lanes_gate": {"status": "pending"}
            }
        },
        "ready_for_merge": {"changed_files": []},
        "ready_for_merge_receipts": [],
        "verification_backlog": {"rows": []},
        "plugin_manifest": {},
        "plugin_flow": {}
    });
    let errors = aggregate_errors_at(&root, base.clone());
    assert!(
        errors.contains(&"goal_binding_receipt_stale_or_mismatched".to_string()),
        "{errors:?}"
    );
    assert!(
        errors.contains(&"post_merge_receipt_not_lane_bound".to_string()),
        "{errors:?}"
    );

    let mut blocked_status = base;
    blocked_status["completion_manifest"]["goal_binding"]["get_goal_receipt"]["observed_contract_path"] =
        json!("GOAL_CONTRACT.md");
    blocked_status["completion_manifest"]["goal_binding"]["get_goal_receipt"]["observed_status"] =
        json!("blocked");
    let errors = aggregate_errors_at(&root, blocked_status);
    assert!(
        errors.contains(&"goal_binding_receipt_stale_or_mismatched".to_string()),
        "{errors:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup aggregate root verification stage");
}
