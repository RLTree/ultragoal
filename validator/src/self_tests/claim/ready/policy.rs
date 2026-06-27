use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn included_manifest() -> Value {
    json!({"claims":[{"id":"C1","status":"pass","claim_ceiling_effect":"included"}]})
}

fn lane() -> Value {
    json!({
        "id":"lane-1",
        "workspace":"w",
        "branch":"b",
        "base_commit":"base",
        "current_commit":"head",
        "target_branch":"main",
        "target_head_at_launch":"t0",
        "target_head_at_validation":"t1",
        "merge_base_at_validation":"mb",
        "execplan":"plan.md",
        "claim_ids":["C1"],
        "owned_paths":["src"]
    })
}

fn ready() -> Value {
    json!({
        "lane_id":"lane-1",
        "ready":true,
        "blocked_reasons":[],
        "worktree_clean":true,
        "teardown_ready":true,
        "commands":[{"exit":0}],
        "workspace":"w",
        "branch":"b",
        "base_commit":"base",
        "commit":"head",
        "target_branch":"main",
        "target_head_at_launch":"t0",
        "target_head_at_validation":"t1",
        "merge_base":"mb",
        "execplan":"plan.md",
        "claim_ids":["C1"],
        "changed_files":["src/lib.rs"],
        "validator_run_id":"run",
        "provenance":{"validator_run_id":"run"}
    })
}

#[test]
fn ready_join_reports_readiness_lane_and_claim_binding_failures() {
    let cm = included_manifest();
    let mut out = Vec::<Failure>::new();
    let bad_ready = json!({
        "lane_id":"missing",
        "ready":false,
        "blocked_reasons":["blocked"],
        "worktree_clean":false,
        "teardown_ready":false,
        "commands":[]
    });
    crate::claim_semantics::ready::join::check_ready_join(
        &cm,
        &json!({"lanes":[]}),
        &bad_ready,
        &mut out,
    );
    let got = errors(&out);
    for expected in [
        "ready_receipt_not_ready",
        "ready_receipt_has_blockers",
        "dirty_self_reported_clean",
        "stale_worktree_after_closeout",
        "command_receipt_missing_or_failed",
        "ready_receipt_not_lane_bound",
    ] {
        assert!(got.contains(&expected), "{expected}: {got:?}");
    }

    out.clear();
    let mut mismatched = ready();
    mismatched["workspace"] = json!("other");
    crate::claim_semantics::ready::join::check_ready_join(
        &cm,
        &json!({"lanes":[lane()]}),
        &mismatched,
        &mut out,
    );
    assert!(errors(&out).contains(&"ready_receipt_not_lane_bound"));

    out.clear();
    let mut claim_mismatch = ready();
    claim_mismatch["claim_ids"] = json!(["OTHER"]);
    claim_mismatch["changed_files"] = json!(["docs/outside.md"]);
    crate::claim_semantics::ready::join::check_ready_join(
        &cm,
        &json!({"lanes":[lane()]}),
        &claim_mismatch,
        &mut out,
    );
    assert_eq!(
        errors(&out)
            .into_iter()
            .filter(|error| *error == "ready_receipt_not_lane_bound")
            .count(),
        2
    );
}

#[test]
fn ready_receipt_runtime_and_generated_artifact_checks_fail_closed() {
    let root = crate::self_tests::boundaries::support::temp_root("ready-receipt");
    let mut out = Vec::<Failure>::new();
    let bundle = json!({
        "validator_receipt": {
            "status":"pass",
            "run_id":"run",
            "validator_execution": {
                "command":{"command":"ultragoal source audit"},
                "validator_artifacts":[{"path":"validator","digest":crate::self_tests::boundaries::support::sha('a')}]
            },
            "generated_artifacts":[]
        }
    });
    let bad_ready = json!({
        "lane_id":"lane-1",
        "ready":true,
        "validator_run_id":"other",
        "provenance":{"validator_run_id":"different"},
        "commands":[{"command":"not_run coverage","exit":0}]
    });
    let mut expected = BTreeMap::new();
    expected.insert(
        "validator".to_string(),
        crate::self_tests::boundaries::support::sha('b'),
    );
    expected.insert(
        "missing".to_string(),
        crate::self_tests::boundaries::support::sha('c'),
    );
    crate::claim_semantics::ready::receipt::check_validator_receipt(
        &bundle, &bad_ready, &expected, &mut out,
    );
    let got = errors(&out);
    for expected in [
        "not_run_is_not_evidence",
        "validator_receipt_not_runtime_provenance",
        "validator_artifact_not_current",
        "ready_receipt_not_validator_output",
        "validator_command_missing_full_argv",
    ] {
        assert!(got.contains(&expected), "{expected}: {got:?}");
    }

    out.clear();
    crate::claim_semantics::ready::receipt::check_ready_receipt_set(
        &bundle,
        &[ready(), ready()],
        &mut out,
    );
    assert!(errors(&out).contains(&"ready_receipt_not_lane_bound"));

    let receipt_path = root.join("generated/ready.json");
    let receipt = ready();
    write_json(&receipt_path, &receipt);
    let digest = crate::digest::file(&receipt_path).expect("digest");
    let with_artifact = json!({
        "validator_receipt": {
            "generated_artifacts":[{
                "artifact_type":"ready_for_merge",
                "validator_run_id":"run",
                "path":"generated/ready.json",
                "digest":digest
            }]
        }
    });
    assert!(
        crate::claim_semantics::ready::receipt::generated_ready_artifact_ok(
            &with_artifact,
            &receipt,
            &root
        )
    );
    let bad_artifact = json!({
        "validator_receipt": {
            "generated_artifacts":[{
                "artifact_type":"ready_for_merge",
                "validator_run_id":"run",
                "path":"../escape.json",
                "digest":crate::digest::ZERO
            }]
        }
    });
    assert!(
        !crate::claim_semantics::ready::receipt::generated_ready_artifact_ok(
            &bad_artifact,
            &receipt,
            &root
        )
    );
    let malformed_path = root.join("generated/malformed.json");
    std::fs::write(&malformed_path, b"{").expect("malformed ready artifact");
    let malformed_digest = crate::digest::file(&malformed_path).expect("malformed digest");
    let malformed_artifact = json!({
        "validator_receipt": {
            "generated_artifacts":[{
                "artifact_type":"ready_for_merge",
                "validator_run_id":"run",
                "path":"generated/malformed.json",
                "digest":malformed_digest
            }]
        }
    });
    assert!(
        !crate::claim_semantics::ready::receipt::generated_ready_artifact_ok(
            &malformed_artifact,
            &receipt,
            &root
        )
    );
    std::fs::remove_dir_all(root).expect("cleanup ready receipt");
}
