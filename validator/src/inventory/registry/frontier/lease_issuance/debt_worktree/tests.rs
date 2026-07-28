use super::test_fixture::{Fixture, empty_digest, git, path_digest};
use super::validate as validate_with_reads;
use crate::context::{BuildRequest, LiveContext};
use crate::inventory::types::InventoryError;
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn validate(
    records: &[Value],
    registry: &Value,
    base: (&str, &str),
    root: &Path,
) -> Result<(), InventoryError> {
    let context = LiveContext::build(BuildRequest::new(root)).unwrap();
    let reads = context.begin_read_session().unwrap();
    validate_with_reads(&reads, records, registry, base, root)
}

#[test]
fn disjoint_exact_p0_worktrees_are_accepted() {
    let fixture = Fixture::new();
    validate(
        &fixture.records(),
        &fixture.registry(),
        fixture.base(),
        &fixture.root,
    )
    .unwrap();
}

#[test]
fn repository_fsmonitor_configuration_cannot_execute_during_validation() {
    let fixture = Fixture::new();
    let marker = fixture.root.parent().unwrap().join("fsmonitor-marker");
    let monitor = fixture.root.parent().unwrap().join("fsmonitor");
    fs::write(
        &monitor,
        format!(
            "#!/bin/sh\nprintf invoked > '{}'\nprintf '\\n'\n",
            marker.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&monitor, fs::Permissions::from_mode(0o700)).unwrap();
    git(
        &fixture.root,
        &["config", "core.fsmonitor", monitor.to_str().unwrap()],
    );

    validate(
        &fixture.records(),
        &fixture.registry(),
        fixture.base(),
        &fixture.root,
    )
    .unwrap();
    assert!(!marker.exists());
}

#[test]
fn overlapping_stale_forbidden_and_uncovered_paths_are_rejected() {
    let fixture = Fixture::new();
    let mut overlap = fixture.records();
    overlap[1]["support_files"] = json!(["src/fit_support.rs"]);
    overlap[1]["owned_files"] = json!(["src/routine.rs", "src/fit_support.rs"]);
    assert!(validate(&overlap, &fixture.registry(), fixture.base(), &fixture.root).is_err());

    let mut stale = fixture.records();
    stale[0]["diagnostic_path_set_digest"] = json!(empty_digest());
    assert!(validate(&stale, &fixture.registry(), fixture.base(), &fixture.root).is_err());

    let mut forbidden = fixture.records();
    forbidden[0]["diagnostic_paths"] = json!(["schemas/shared.json"]);
    forbidden[0]["owned_files"] = json!(["schemas/shared.json", "src/fit_support.rs"]);
    forbidden[0]["diagnostic_path_set_digest"] = json!(path_digest("schemas/shared.json"));
    assert!(
        validate(
            &forbidden,
            &fixture.registry(),
            fixture.base(),
            &fixture.root
        )
        .is_err()
    );

    assert!(
        validate(
            &fixture.records()[..1],
            &fixture.registry(),
            fixture.base(),
            &fixture.root
        )
        .is_err()
    );
}

#[test]
fn p0_worktrees_require_blocked_product_and_current_diagnostics() {
    let fixture = Fixture::new();
    let records = fixture.records();
    let mut eligible = fixture.registry();
    eligible["pre_adoption_source"]["eligible_scheduler_nodes"] = json!(["N14"]);
    assert!(validate(&records, &eligible, fixture.base(), &fixture.root).is_err());

    let mut active = fixture.registry();
    active["lanes"][0]["state"] = json!("rework");
    assert!(validate(&records, &active, fixture.base(), &fixture.root).is_err());

    let mut stale = fixture.registry();
    stale["lease_state"]["p0_exception"]["debt_path_sources"]["clippy"]["candidate_tree"] =
        json!("3333333333333333333333333333333333333333");
    assert!(validate(&records, &stale, fixture.base(), &fixture.root).is_err());

    let mut incomplete = fixture.registry();
    incomplete["lease_state"]["p0_exception"]["issuance_transition"]["source_sets"]["clippy"] =
        json!(["src/fit.rs"]);
    assert!(validate(&records, &incomplete, fixture.base(), &fixture.root).is_err());
}

#[test]
fn p0_worktree_identity_rejects_missing_wrong_branch_changed_head_and_dirty_state() {
    let fixture = Fixture::new();
    let mut missing = fixture.records();
    missing[0]["worktree"] = json!(fixture.root.join("missing"));
    assert!(validate(&missing, &fixture.registry(), fixture.base(), &fixture.root).is_err());

    let mut wrong_branch = fixture.records();
    wrong_branch[0]["branch"] = json!("codex/p0-other");
    assert!(
        validate(
            &wrong_branch,
            &fixture.registry(),
            fixture.base(),
            &fixture.root
        )
        .is_err()
    );

    git(
        &fixture.fit,
        &["commit", "--allow-empty", "-q", "-m", "changed"],
    );
    assert!(
        validate(
            &fixture.records(),
            &fixture.registry(),
            fixture.base(),
            &fixture.root
        )
        .is_err()
    );
    git(
        &fixture.fit,
        &["reset", "--hard", "-q", fixture.commit.as_str()],
    );
    fs::write(fixture.fit.join("dirty"), b"dirty").unwrap();
    assert!(
        validate(
            &fixture.records(),
            &fixture.registry(),
            fixture.base(),
            &fixture.root
        )
        .is_err()
    );
}
