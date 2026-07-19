use super::{digest, validate};
use serde_json::{Value, json};
use std::collections::BTreeSet;

const COMMIT: &str = "1111111111111111111111111111111111111111";
const TREE: &str = "2222222222222222222222222222222222222222";

#[test]
fn disjoint_exact_p0_worktrees_are_accepted() {
    let records = vec![
        record("fit", "src/fit.rs", "src/fit_support.rs"),
        record("routine", "src/routine.rs", "src/routine_support.rs"),
    ];
    validate(&records, &registry(), (COMMIT, TREE)).unwrap();
}

#[test]
fn overlapping_p0_worktrees_are_rejected() {
    let mut records = vec![
        record("fit", "src/fit.rs", "src/fit_support.rs"),
        record("routine", "src/routine.rs", "src/routine_support.rs"),
    ];
    records[1]["support_files"] = json!(["src/fit_support.rs"]);
    records[1]["owned_files"] = json!(["src/routine.rs", "src/fit_support.rs"]);
    assert!(validate(&records, &registry(), (COMMIT, TREE)).is_err());
}

#[test]
fn stale_path_digest_is_rejected() {
    let mut records = vec![
        record("fit", "src/fit.rs", "src/fit_support.rs"),
        record("routine", "src/routine.rs", "src/routine_support.rs"),
    ];
    records[0]["diagnostic_path_set_digest"] =
        json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    assert!(validate(&records, &registry(), (COMMIT, TREE)).is_err());
}

#[test]
fn forbidden_or_uncovered_paths_are_rejected() {
    let mut blocked = vec![
        record("fit", "src/fit.rs", "src/fit_support.rs"),
        record("routine", "src/routine.rs", "src/routine_support.rs"),
    ];
    blocked[0]["diagnostic_paths"] = json!(["schemas/shared.json"]);
    blocked[0]["owned_files"] = json!(["schemas/shared.json", "src/fit_support.rs"]);
    blocked[0]["diagnostic_path_set_digest"] = json!(path_digest("schemas/shared.json"));
    assert!(validate(&blocked, &registry(), (COMMIT, TREE)).is_err());

    let incomplete = vec![record("fit", "src/fit.rs", "src/fit_support.rs")];
    assert!(validate(&incomplete, &registry(), (COMMIT, TREE)).is_err());
}

#[test]
fn p0_worktrees_are_rejected_when_product_work_is_eligible_or_active() {
    let records = vec![
        record("fit", "src/fit.rs", "src/fit_support.rs"),
        record("routine", "src/routine.rs", "src/routine_support.rs"),
    ];
    let mut eligible = registry();
    eligible["pre_adoption_source"]["eligible_scheduler_nodes"] = json!(["N14"]);
    assert!(validate(&records, &eligible, (COMMIT, TREE)).is_err());

    let mut active = registry();
    active["lanes"][0]["state"] = json!("rework");
    assert!(validate(&records, &active, (COMMIT, TREE)).is_err());

    let mut product_record = record("product", "src/fit.rs", "src/fit_support.rs");
    product_record["exception_id"] = Value::Null;
    product_record["lane_id"] = json!("N14");
    assert!(
        validate(
            &[records[0].clone(), product_record],
            &registry(),
            (COMMIT, TREE)
        )
        .is_err()
    );
}

fn registry() -> Value {
    json!({
        "pre_adoption_source": {"eligible_scheduler_nodes": []},
        "lanes": [{"id": "N14", "state": "blocked"}],
        "lease_state": {
            "p0_exception": {
                "allowed": {"paths": [
                    "src/fit.rs",
                    "src/fit_support.rs",
                    "src/routine.rs",
                    "src/routine_support.rs"
                ]},
                "forbidden": {"paths": ["schemas/**"]}
            }
        }
    })
}

fn record(name: &str, diagnostic: &str, dependency_path: &str) -> Value {
    json!({
        "lease_id": format!("P0-{name}"),
        "lane_id": "P0",
        "exception_id": "P0-DEBT-REPAIR",
        "base_commit": COMMIT,
        "base_tree": TREE,
        "branch": format!("codex/p0-{name}"),
        "worktree": format!("/worktrees/{name}"),
        "status": "issued",
        "diagnostic_paths": [diagnostic],
        "support_files": [dependency_path],
        "owned_files": [diagnostic, dependency_path],
        "diagnostic_path_set_digest": path_digest(diagnostic)
    })
}

fn path_digest(path: &str) -> String {
    digest(&BTreeSet::from([path.to_owned()]))
}
