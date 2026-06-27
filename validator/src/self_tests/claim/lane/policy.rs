use crate::audit::contract::Failure;
use serde_json::json;

#[test]
fn lane_policy_validates_clock_size_and_dependency_dispatch() {
    let root = crate::self_tests::boundaries::support::temp_root("lane-policy-dispatch");
    let mut failures = Vec::<Failure>::new();
    let run_at = crate::claim_semantics::lane::policy::validation_clock(
        &json!({"automation_tick_receipt":{"checked_at":"2026-06-25T00:10:00Z"}}),
        &mut failures,
    );
    assert!(run_at > 0);

    let size = json!({
        "owned_path_groups":["validator","fixtures","docs"],
        "work_units":["schema","validator","fixture","receipt"],
        "independent_outcome":"This lane owns a separable authority path with enough evidence to validate independently from the parent.",
        "why_not_parent_inline":"The authority surface has isolated evidence and rerun commands.",
        "why_not_smaller":"Splitting smaller would separate fixtures from the validator branch."
    });
    let registry = json!({
        "lanes":[
            {
                "id":"down",
                "status":"active",
                "current_commit":"down-head",
                "workspace":"worktrees/down",
                "lane_size_evidence":size,
                "dependencies":[{
                    "upstream_lane_id":"up",
                    "claim_id":"CLAIM-UP",
                    "required_status":"merged",
                    "validated_status":"merged",
                    "validated_at":"2026-06-25T00:09:00Z",
                    "upstream_commit":"wrong-head",
                    "evidence_digest":crate::self_tests::boundaries::support::sha('1'),
                    "upstream_ready_receipt":{"path":"ready/up.json","digest":crate::self_tests::boundaries::support::sha('2')}
                }]
            },
            {
                "id":"up",
                "status":"merged",
                "current_commit":"up-head",
                "workspace":"worktrees/up",
                "lane_size_evidence":size
            }
        ],
        "root_verification_phases":{
            "pre_merge_lane_gate":{"status":"pass"},
            "post_merge_integration_gate":{"status":"pass"},
            "final_all_lanes_gate":{"status":"pending"}
        }
    });
    crate::claim_semantics::lane::policy::check_lanes(
        &json!({"validator_receipt":{"generated_artifacts":[]}}),
        &registry,
        &json!({"changed_files":[]}),
        &[],
        &root,
        run_at,
        &mut failures,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.error == "dependency_claim_not_proven"),
        "{failures:?}"
    );

    failures.clear();
    crate::claim_semantics::lane::policy::check_lanes(
        &json!({}),
        &json!({"lanes":[{
            "id":"tiny",
            "lane_size_evidence":{
                "owned_path_groups":[],
                "work_units":[],
                "independent_outcome":"",
                "why_not_parent_inline":"",
                "why_not_smaller":""
            }
        }]}),
        &json!({}),
        &[],
        &root,
        run_at,
        &mut failures,
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.error == "lane_too_small_or_overlapping"),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(root);
}
