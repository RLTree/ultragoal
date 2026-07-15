use std::fs;
use std::os::unix::fs::{FileTypeExt, PermissionsExt, symlink};

use super::routine_work::{
    LocalDirtyTree, PlanRequest, ProductionRoutineIssuer, RoutineAdapterSpec, RoutineCancellation,
    RoutineMediatorStatus, RoutineReuseInput, mediate_prepared_routine_execution_production,
    plan_routine, prepare_routine_execution,
};
use super::scenario::{TempRepo, graph};

fn authority_root(label: &str) -> (TempRepo, std::path::PathBuf) {
    let repo = TempRepo::new(label);
    let root = repo.root().join("authority");
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    (repo, root)
}

fn state(root: &std::path::Path) -> Vec<u8> {
    fs::read(root.join("routine-authority.state")).unwrap()
}

#[test]
fn authority_root_and_state_aliases_or_special_objects_refuse_without_rewrite() {
    let (mut repo, root) = authority_root("ledger-object-security");
    ProductionRoutineIssuer::open(&root).unwrap();
    let exact = state(&root);
    let state_path = root.join("routine-authority.state");
    let alias = repo.root().join("state-hardlink");
    fs::hard_link(&state_path, &alias).unwrap();
    assert!(ProductionRoutineIssuer::open(&root).is_err());
    assert_eq!(fs::read(&state_path).unwrap(), exact);
    fs::remove_file(alias).unwrap();

    let real = repo.root().join("real-authority");
    fs::create_dir(&real).unwrap();
    fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
    let linked = repo.root().join("linked-authority");
    symlink(&real, &linked).unwrap();
    assert!(ProductionRoutineIssuer::open(&linked).is_err());
    assert!(!real.join("routine-authority.state").exists());

    let fifo = repo.root().join("authority-fifo");
    let fifo_c = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o600) }, 0);
    assert!(ProductionRoutineIssuer::open(&fifo).is_err());
    assert_eq!(
        fs::symlink_metadata(&fifo).unwrap().file_type().is_fifo(),
        true
    );
    repo.teardown_after_assertions();
}

#[test]
fn exact_noop_bypasses_authority_initialization_and_workspace_writes() {
    let mut repo = TempRepo::new("production-noop-zero-write");
    let context = repo.context("routine-noop");
    let snapshot = LocalDirtyTree::capture(&context).unwrap();
    let graph = graph();
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).unwrap();
    let prepared = prepare_routine_execution(
        &context,
        &graph,
        &snapshot,
        &plan,
        RoutineAdapterSpec::new("routine", Vec::new()),
    )
    .unwrap();
    let before = repo.tree();
    let authority = repo.root().join("absent-authority");
    let result = mediate_prepared_routine_execution_production(
        &authority,
        &context,
        &plan,
        prepared,
        None,
        RoutineCancellation::new(),
        RoutineReuseInput::default(),
    )
    .unwrap();
    assert_eq!(result.status(), RoutineMediatorStatus::CompleteNoOp);
    assert!(!authority.exists());
    assert_eq!(repo.tree(), before);
    repo.teardown_after_assertions();
}

#[test]
fn authenticated_state_truncate_unknown_duplicate_and_reorder_mutations_refuse() {
    let (mut repo, root) = authority_root("ledger-state-mutations");
    ProductionRoutineIssuer::open(&root).unwrap();
    let state_path = root.join("routine-authority.state");
    let exact = state(&root);
    let text = String::from_utf8(exact.clone()).unwrap();
    let mutations = [
        exact[..exact.len() / 2].to_vec(),
        text.replacen('{', "{\"unknown\":true,", 1).into_bytes(),
        text.replacen(
            "\"schema_version\":",
            "\"schema_version\":\"duplicate\",\"schema_version\":",
            1,
        )
        .into_bytes(),
        {
            let mut bytes = exact.clone();
            bytes.reverse();
            bytes
        },
    ];
    for mutation in mutations {
        fs::write(&state_path, mutation).unwrap();
        assert!(ProductionRoutineIssuer::open(&root).is_err());
        fs::write(&state_path, &exact).unwrap();
        ProductionRoutineIssuer::open(&root).unwrap();
    }
    assert_eq!(state(&root), exact);
    repo.teardown_after_assertions();
}
