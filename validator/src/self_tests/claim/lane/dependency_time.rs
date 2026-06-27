use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn fixture(root: &Path) -> (Value, Value, Value, Value, Value, Value) {
    std::fs::create_dir_all(root.join("ready")).expect("ready dir");
    let ready = json!({
        "ready": true,
        "lane_id": "up",
        "commit": "up-commit",
        "branch": "up-branch",
        "workspace": "up-workspace",
        "base_commit": "base",
        "claim_ids": ["C"],
        "blocked_reasons": [],
        "worktree_clean": true,
        "teardown_ready": true,
        "generated_at": "2026-06-25T00:20:00Z",
        "validator_run_id": "run"
    });
    let ready_digest = crate::claim_semantics::canonical_digest(&ready).expect("ready digest");
    let ready_path = root.join("ready/up.json");
    write_json(&ready_path, &ready);
    let ready_file_digest = crate::digest::file(&ready_path).expect("ready file digest");
    let upstream = json!({
        "id": "up",
        "status": "merged",
        "current_commit": "up-commit",
        "branch": "up-branch",
        "workspace": "up-workspace",
        "base_commit": "base",
        "claim_ids": ["C"],
        "last_heartbeat": "2026-06-25T00:10:00Z",
        "ready_receipt": {"path": "ready.json", "digest": ready_digest}
    });
    let dep = json!({
        "upstream_lane_id": "up",
        "claim_id": "C",
        "required_status": "merged",
        "validated_status": "merged",
        "validated_at": "2026-06-25T00:40:00Z",
        "upstream_commit": "up-commit",
        "evidence_digest": ready_digest,
        "upstream_ready_receipt": {"path": "ready.json", "digest": ready_digest}
    });
    let post_digest = crate::self_tests::boundaries::support::sha('9');
    let root_phases = json!({"post_merge_integration_gate": {
        "status": "pass",
        "validated_at": "2026-06-25T00:30:00Z",
        "artifact_receipt": {"path": "post.json", "digest": post_digest}
    }});
    let lane = json!({
        "id": "down",
        "dependency_release": {
            "action": "resume",
            "downstream_lane_id": "down",
            "upstream_lane_id": "up",
            "claim_id": "C",
            "post_merge_receipt": {"path": "post.json", "digest": post_digest},
            "decided_at": "2026-06-25T00:40:00Z"
        }
    });
    let bundle = json!({"validator_receipt":{"generated_artifacts":[{
        "artifact_type": "ready_for_merge",
        "validator_run_id": "run",
        "path": "ready/up.json",
        "digest": ready_file_digest
    }]}});
    (lane, dep, upstream, ready, root_phases, bundle)
}

fn run_case(
    lane: &Value,
    dep: &Value,
    upstream: &Value,
    ready: &Value,
    root_phases: &Value,
    bundle: &Value,
    root: &Path,
) -> Vec<String> {
    let mut lanes = BTreeMap::new();
    lanes.insert("up".to_string(), upstream);
    let mut ready_index = BTreeMap::new();
    ready_index.insert("up".to_string(), ready);
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::lane::dependency::check(
        lane,
        dep,
        &lanes,
        &ready_index,
        root_phases,
        bundle,
        root,
        &mut out,
    );
    out.into_iter().map(|failure| failure.error).collect()
}

#[test]
fn lane_dependency_time_and_release_edges_fail_closed() {
    let root = crate::self_tests::boundaries::support::temp_root("lane-dependency-time-edges");
    let (lane, dep, upstream, ready, root_phases, bundle) = fixture(&root);
    assert!(run_case(&lane, &dep, &upstream, &ready, &root_phases, &bundle, &root).is_empty());

    let mut bad_dep_time = dep.clone();
    bad_dep_time["validated_at"] = json!("not-a-time");
    assert!(
        run_case(
            &lane,
            &bad_dep_time,
            &upstream,
            &ready,
            &root_phases,
            &bundle,
            &root
        )
        .contains(&"ready_receipt_not_lane_bound".to_string())
    );

    let mut bad_heartbeat = upstream.clone();
    bad_heartbeat["last_heartbeat"] = json!("not-a-time");
    assert!(
        run_case(
            &lane,
            &dep,
            &bad_heartbeat,
            &ready,
            &root_phases,
            &bundle,
            &root
        )
        .contains(&"ready_receipt_not_lane_bound".to_string())
    );

    let mut bad_ready_time = ready.clone();
    bad_ready_time["generated_at"] = json!("not-a-time");
    assert!(
        run_case(
            &lane,
            &dep,
            &upstream,
            &bad_ready_time,
            &root_phases,
            &bundle,
            &root
        )
        .contains(&"ready_receipt_not_lane_bound".to_string())
    );
    assert!(
        crate::claim_semantics::lane::dependency::dependency_order_bad(
            &dep,
            &upstream,
            &bad_ready_time,
            &root_phases
        )
    );

    let mut bad_post_time = root_phases.clone();
    bad_post_time["post_merge_integration_gate"]["validated_at"] = json!("not-a-time");
    assert!(
        run_case(
            &lane,
            &dep,
            &upstream,
            &ready,
            &bad_post_time,
            &bundle,
            &root
        )
        .contains(&"ready_receipt_not_lane_bound".to_string())
    );

    let mut bad_release = lane.clone();
    bad_release["dependency_release"]["post_merge_receipt"]["digest"] =
        json!(crate::self_tests::boundaries::support::sha('8'));
    assert!(
        run_case(
            &bad_release,
            &dep,
            &upstream,
            &ready,
            &root_phases,
            &bundle,
            &root
        )
        .contains(&"dependency_release_missing".to_string())
    );

    assert!(
        crate::claim_semantics::lane::dependency::dependency_release_bad(
            &lane,
            &dep,
            &bad_post_time
        )
    );
    std::fs::remove_dir_all(root).expect("cleanup lane dependency time edges");
}
