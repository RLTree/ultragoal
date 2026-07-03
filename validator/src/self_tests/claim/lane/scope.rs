use crate::audit::contract::Failure;
use serde_json::{Value, json};

fn errors(out: &[Failure]) -> Vec<String> {
    out.iter().map(|failure| failure.error.clone()).collect()
}

fn active_lanes() -> Vec<Value> {
    vec![
        json!({
            "id":"lane-a",
            "status":"active",
            "owned_paths":["src"],
            "workspace":"work/lane",
            "state_roots":["state/shared"],
            "scratch_roots":["scratch/a"],
            "tool_cache_roots":["cache/a"],
            "browser_profile_roots":["browser/a"],
            "artifact_root":"artifacts/shared",
            "port_allocations":[{"port":7000}],
            "branch":"codex/shared",
            "current_commit":"commit-a",
            "target_branch":"main"
        }),
        json!({
            "id":"lane-b",
            "status":"active",
            "owned_paths":["src/app"],
            "workspace":"work/lane/sub",
            "state_roots":["state/shared/sub"],
            "scratch_roots":["scratch/a/sub"],
            "tool_cache_roots":["cache/a/sub"],
            "browser_profile_roots":["browser/a/sub"],
            "artifact_root":"artifacts/shared/sub",
            "port_allocations":[{"port":7000}],
            "branch":"codex/shared",
            "current_commit":"commit-b",
            "target_branch":"main"
        }),
        json!({
            "id":"lane-c",
            "status":"active",
            "owned_paths":["../escape"],
            "workspace":"../escape"
        }),
    ]
}

#[test]
fn lane_scope_overlap_and_mutable_resources_fail_closed() {
    let lanes = active_lanes();
    let mut out = Vec::new();
    crate::claim_semantics::lane::root::scope::lane_overlap(&lanes, &mut out);
    crate::claim_semantics::lane::isolation::mutable_resources(&lanes, &mut out);
    let got = errors(&out);
    for expected in [
        "invalid_lane_owned_path",
        "active_lane_owned_path_overlap",
        "invalid_lane_resource_path",
        "active_lane_mutable_resource_overlap",
    ] {
        assert!(got.contains(&expected.to_string()), "{expected}: {out:?}");
    }
}

#[test]
fn root_verification_and_parent_changed_file_authority_fail_closed_and_pass() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("lane-root-scope");
    std::fs::create_dir_all(root.join("artifacts")).expect("artifacts");
    let post = root.join("artifacts/post.json");
    std::fs::write(
        &post,
        serde_json::to_vec(&json!({
            "schema":"harness-ultragoal.root-verification-receipt.v1",
            "phase":"post_merge_integration_gate",
            "upstream_lane_id":"lane-a",
            "upstream_commit":"commit-a",
            "target_branch":"main",
            "validated_at":"2026-06-25T00:00:00Z",
            "merge_reachability":{"upstream_commit_reachable":true},
            "commands":[{"id":"post-cmd","exit":0}]
        }))
        .expect("post receipt bytes"),
    )
    .expect("post receipt");
    let digest = crate::digest::file(&post).expect("post digest");
    let lanes = vec![json!({
        "id":"lane-a",
        "status":"active",
        "owned_paths":["src"],
        "current_commit":"commit-a",
        "target_branch":"main"
    })];
    let post_merge_state = json!({
        "phase":"post_merge_integration_gate",
        "status":"pass",
        "validated_at":"2026-06-25T00:00:00Z",
        "command_receipt":{"id":"post-cmd","exit":0,"artifact_path":"artifacts/post.json","artifact_digest":digest},
        "artifact_receipt":{"path":"artifacts/post.json","digest":digest}
    });
    let verification_states = json!({
        "pre_merge_lane_gate":{"status":"pending"},
        "post_merge_integration_gate":post_merge_state,
        "final_all_lanes_gate":{"phase":"final_all_lanes_gate","status":"pass"}
    });
    let mut out = Vec::new();
    crate::claim_semantics::lane::root::scope::root_verification_states(
        &verification_states,
        &lanes,
        &root,
        &mut out,
    );
    let got = errors(&out);
    for expected in [
        "post_merge_without_pre_merge_receipt",
        "final_gate_with_nonterminal_lanes",
    ] {
        assert!(got.contains(&expected.to_string()), "{expected}: {out:?}");
    }
    assert!(
        !got.contains(&"post_merge_receipt_not_lane_bound".to_string()),
        "{out:?}"
    );

    let no_post = json!({
        "pre_merge_lane_gate":{"status":"pass"},
        "post_merge_integration_gate":{"status":"pending"},
        "final_all_lanes_gate":{"phase":"final_all_lanes_gate","status":"pass"}
    });
    out.clear();
    crate::claim_semantics::lane::root::scope::root_verification_states(
        &no_post, &lanes, &root, &mut out,
    );
    assert!(errors(&out).contains(&"final_gate_without_post_merge_receipt".to_string()));

    out.clear();
    crate::claim_semantics::lane::root::scope::parent_changed_files_are_registered(
        &lanes,
        &json!({"changed_files":["docs/unowned.md"]}),
        &mut out,
    );
    assert!(errors(&out).contains(&"parent_diff_without_registered_lane".to_string()));
    std::fs::remove_dir_all(root).expect("cleanup lane root scope");
}
