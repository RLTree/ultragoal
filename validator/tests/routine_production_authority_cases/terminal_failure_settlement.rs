use super::*;

#[test]
pub(crate) fn failure_and_cancellation_settle_terminally_without_recovery() {
    let failed_fixture = fixture("production-failed", true);
    let failed_authority = AuthorityRoot::new("production-failed");
    let failed = mediate(
        &failed_authority,
        &failed_fixture,
        prepared_failure(&failed_fixture),
        Vec::new(),
    )
    .unwrap();
    assert_eq!(failed.status(), RoutineMediatorStatus::IncompleteExecution);
    let failed_issuer = ProductionRoutineIssuer::open(failed_authority.path()).unwrap();
    assert!(
        failed_issuer
            .pending_recovery(
                &failed_fixture.context,
                &failed_fixture.plan,
                &prepared_failure(&failed_fixture),
            )
            .unwrap()
            .is_none()
    );

    let cancelled_fixture = fixture("production-cancelled", true);
    let cancelled_authority = AuthorityRoot::new("production-cancelled");
    let cancellation = RoutineCancellation::new();
    cancellation.cancel();
    let cancelled = mediate_prepared_routine_execution_production(
        cancelled_authority.path(),
        &cancelled_fixture.context,
        &cancelled_fixture.plan,
        prepared(&cancelled_fixture),
        None,
        cancellation,
        RoutineReuseInput::new(Vec::new()),
    )
    .unwrap();
    assert_eq!(cancelled.status(), RoutineMediatorStatus::Cancelled);
    let cancelled_issuer = ProductionRoutineIssuer::open(cancelled_authority.path()).unwrap();
    assert!(
        cancelled_issuer
            .pending_recovery(
                &cancelled_fixture.context,
                &cancelled_fixture.plan,
                &prepared(&cancelled_fixture),
            )
            .unwrap()
            .is_none()
    );
}

#[test]
pub(crate) fn stale_request_and_self_consistent_substitution_refuse_without_hidden_writes() {
    let stale_fixture = fixture("production-stale", true);
    let authority = AuthorityRoot::new("production-stale");
    let stale = prepared(&stale_fixture);
    stale_fixture
        .repo
        .write("src/late.rs", b"pub fn late() {}\n");
    let before_target = stale_fixture.repo.tree();
    let before_authority = authority.tree();
    let refused = mediate(&authority, &stale_fixture, stale, Vec::new()).unwrap_err();
    assert!(refused.cause().contains("stale") || refused.cause().contains("mutated"));
    assert_eq!(stale_fixture.repo.tree(), before_target);
    assert_eq!(authority.tree(), before_authority);

    let original = fixture("production-substitution-original", true);
    let substitute = fixture("production-substitution-coherent", true);
    let authority = AuthorityRoot::new("production-substitution");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let original_request = prepared(&original);
    issuer
        .test_reserve_and_abandon(&original.context, &original.plan, &original_request, true)
        .unwrap();
    let recovery = issuer
        .pending_recovery(&original.context, &original.plan, &original_request)
        .unwrap()
        .unwrap();
    let before_original = original.repo.tree();
    let before_substitute = substitute.repo.tree();
    let before_authority = authority.tree();
    let refused = issuer
        .mediate(
            &substitute.context,
            &substitute.plan,
            prepared(&substitute),
            Some(recovery),
            RoutineCancellation::new(),
            RoutineReuseInput::new(Vec::new()),
        )
        .unwrap_err();
    assert_eq!(
        refused.cause(),
        "routine-production-recovery-authority-stale"
    );
    assert_eq!(original.repo.tree(), before_original);
    assert_eq!(substitute.repo.tree(), before_substitute);
    assert_eq!(authority.tree(), before_authority);
}

#[test]
pub(crate) fn owner_only_store_rejects_unknown_hardlink_symlink_special_and_root_replacement() {
    mutation_case("unknown", |root| {
        fs::write(root.join("unknown"), b"x").unwrap()
    });
    mutation_case("key-hardlink", |root| {
        fs::hard_link(root.join("routine-authority.key"), root.join("key-alias")).unwrap();
    });
    mutation_case("state-symlink", |root| {
        let state = root.join("routine-authority.state");
        let saved = root.join("outside-state");
        fs::rename(&state, &saved).unwrap();
        symlink(&saved, &state).unwrap();
    });
    mutation_case("state-fifo", |root| {
        let state = root.join("routine-authority.state");
        fs::remove_file(&state).unwrap();
        let name = std::ffi::CString::new(state.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    });
    mutation_case("lock-truncate", |root| {
        fs::write(root.join("routine-authority.lock"), b"").unwrap();
    });
    mutation_case("root-mode", |root| {
        fs::set_permissions(root, fs::Permissions::from_mode(0o755)).unwrap();
    });

    let fixture = fixture("production-root-replaced", true);
    let authority = AuthorityRoot::new("production-root-replaced");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let displaced = authority.parent.join("displaced");
    fs::rename(authority.path(), &displaced).unwrap();
    fs::create_dir(authority.path()).unwrap();
    fs::set_permissions(authority.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let error = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert_eq!(error.cause(), "routine-production-authority-root-replaced");
}

pub(crate) fn mutation_case(label: &str, mutate: impl FnOnce(&Path)) {
    let fixture = fixture(&format!("production-{label}"), true);
    let authority = AuthorityRoot::new(&format!("production-{label}"));
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    mutate(authority.path());
    let error = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert!(
        error.cause().starts_with("routine-production-authority-"),
        "unexpected diagnostic: {error}"
    );
    assert!(
        !error
            .to_string()
            .contains(&authority.path().to_string_lossy().as_ref())
    );
}

#[test]
pub(crate) fn authenticated_state_rejects_truncate_unknown_duplicate_reorder_rollback_and_mutate_restore()
 {
    state_mutation_case("truncate", |bytes| bytes.truncate(bytes.len() / 2));
    state_mutation_case("unknown", |bytes| {
        let mut value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        value["unknown"] = serde_json::json!(true);
        *bytes = serde_json::to_vec(&value).unwrap();
    });
    state_mutation_case("reorder", |bytes| bytes.reverse());
    state_mutation_case("duplicate", |bytes| {
        let text = String::from_utf8(bytes.clone()).unwrap();
        *bytes = text.replacen("{", "{\"payload\":null,", 1).into_bytes();
    });

    let rollback_fixture = fixture("production-rollback", true);
    let authority = AuthorityRoot::new("production-rollback");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let state_path = authority.path().join("routine-authority.state");
    let old = fs::read(&state_path).unwrap();
    let request = prepared(&rollback_fixture);
    issuer
        .test_reserve_and_abandon(
            &rollback_fixture.context,
            &rollback_fixture.plan,
            &request,
            false,
        )
        .unwrap();
    fs::write(&state_path, old).unwrap();
    let rollback = issuer
        .pending_recovery(&rollback_fixture.context, &rollback_fixture.plan, &request)
        .unwrap_err();
    assert_eq!(
        rollback.cause(),
        "routine-production-authority-rollback-detected"
    );

    let fixture = fixture("production-mutate-restore", true);
    let authority = AuthorityRoot::new("production-mutate-restore");
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let state_path = authority.path().join("routine-authority.state");
    let original = fs::read(&state_path).unwrap();
    fs::write(&state_path, b"mutated").unwrap();
    fs::write(&state_path, original).unwrap();
    let restored = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert_eq!(
        restored.cause(),
        "routine-production-authority-rollback-detected"
    );
}

pub(crate) fn state_mutation_case(label: &str, mutate: impl FnOnce(&mut Vec<u8>)) {
    let fixture = fixture(&format!("production-state-{label}"), true);
    let authority = AuthorityRoot::new(&format!("production-state-{label}"));
    let issuer = ProductionRoutineIssuer::open(authority.path()).unwrap();
    let request = prepared(&fixture);
    issuer
        .test_reserve_and_abandon(&fixture.context, &fixture.plan, &request, false)
        .unwrap();
    let state = authority.path().join("routine-authority.state");
    let mut bytes = fs::read(&state).unwrap();
    mutate(&mut bytes);
    fs::write(&state, bytes).unwrap();
    let error = issuer
        .pending_recovery(&fixture.context, &fixture.plan, &request)
        .unwrap_err();
    assert!(
        error
            .cause()
            .starts_with("routine-production-authority-state-")
    );
}
