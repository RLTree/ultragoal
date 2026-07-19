use super::lane_binding::{load_declared_dependency_identities, load_exact_dependency_identities};
use crate::context::CandidateIdentity;

const LANES: &[u8] = include_bytes!("../../../../LANE_REGISTRY.json");

#[test]
fn current_external_blocker_and_dependency_identities_are_accepted() {
    let identities = load_declared_dependency_identities(LANES).unwrap();
    assert_eq!(identities.len(), 7);
}

#[test]
fn authority_and_identity_mutations_fail_closed() {
    let original: serde_json::Value = serde_json::from_slice(LANES).unwrap();
    for mutation in 0..5 {
        let mut value = original.clone();
        let lanes = value["lanes"].as_array_mut().unwrap();
        match mutation {
            0 => {
                lane(lanes, "N11")["outcome"]["execution_outcome"] =
                    serde_json::json!("demonstrated")
            }
            1 => lane(lanes, "N11")["current_identity"]["tree"] = serde_json::json!("A".repeat(40)),
            2 => {
                lane(lanes, "N12")["state"] = serde_json::json!("ready");
            }
            3 => {
                lane(lanes, "N12")["dependencies"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            _ => {
                let duplicate = lanes
                    .iter()
                    .find(|lane| lane["id"] == "N03")
                    .unwrap()
                    .clone();
                lanes.push(duplicate);
            }
        }
        assert!(load_declared_dependency_identities(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

fn lane<'a>(lanes: &'a mut [serde_json::Value], id: &str) -> &'a mut serde_json::Value {
    lanes.iter_mut().find(|lane| lane["id"] == id).unwrap()
}

#[test]
fn exact_root_staged_state_is_accepted_without_promoting_n11() {
    let mut value: serde_json::Value = serde_json::from_slice(LANES).unwrap();
    let n12 = value["lanes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|lane| lane["id"] == "N12")
        .unwrap();
    n12["state"] = serde_json::json!("integrating");
    n12["ceiling"] = serde_json::json!("source_accepted");
    n12["current_identity"] = serde_json::json!({
        "lane_id": "N12",
        "commit": "0123456789abcdef0123456789abcdef01234567",
        "tree": "89abcdef0123456789abcdef0123456789abcdef"
    });
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(load_declared_dependency_identities(&bytes).is_ok());
    assert!(load_exact_dependency_identities(&bytes, &candidate(false)).is_ok());
    for invalid in [
        different_commit(),
        different_tree(),
        candidate(true),
        missing_commit(),
        missing_tree(),
    ] {
        assert!(load_exact_dependency_identities(&bytes, &invalid).is_err());
    }
    assert!(load_exact_dependency_identities(LANES, &candidate(false)).is_err());
}

fn candidate(dirty: bool) -> CandidateIdentity {
    CandidateIdentity {
        head_commit: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
        head_tree: Some("89abcdef0123456789abcdef0123456789abcdef".to_owned()),
        branch: Some("codex/test".to_owned()),
        status_sha256: "status".to_owned(),
        worktree_diff_sha256: "worktree".to_owned(),
        staged_diff_sha256: "staged".to_owned(),
        untracked_content_sha256: "untracked".to_owned(),
        dirty,
    }
}

fn different_commit() -> CandidateIdentity {
    let mut value = candidate(false);
    value.head_commit = Some("1123456789abcdef0123456789abcdef01234567".to_owned());
    value
}

fn different_tree() -> CandidateIdentity {
    let mut value = candidate(false);
    value.head_tree = Some("19abcdef0123456789abcdef0123456789abcdef".to_owned());
    value
}

fn missing_commit() -> CandidateIdentity {
    let mut value = candidate(false);
    value.head_commit = None;
    value
}

fn missing_tree() -> CandidateIdentity {
    let mut value = candidate(false);
    value.head_tree = None;
    value
}
