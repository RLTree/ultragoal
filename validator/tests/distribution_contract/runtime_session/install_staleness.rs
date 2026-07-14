#[cfg(unix)]
fn runtime_plan(
    label: &str,
    args: Vec<String>,
) -> (
    JourneyFixture,
    PackageSnapshot,
    InstallTransaction,
    PathBuf,
    RuntimeProbePlan,
) {
    let (fixture, package, installed, executable, host, binding) = runtime_fixture(label);
    let plan = RuntimeProbePlan::from_installed_package(
        binding,
        &host,
        installed.snapshot(),
        &mut ScopedInstall::new(fixture.confined()),
        &package,
        &executable,
        args,
        Duration::from_secs(10),
    )
    .unwrap();
    (fixture, package, installed, executable, plan)
}

#[cfg(unix)]
#[test]
fn runtime_plan_rejects_installed_archive_deleted_after_plan() {
    let (fixture, _package, _installed, _executable, plan) =
        runtime_plan("runtime-install-deleted-after-plan", valid_args());
    std::fs::remove_file(fixture.root.join("plugins/harness-ultragoal.hugpkg")).unwrap();
    assert_eq!(
        plan.execute_bound().unwrap_err().id(),
        ErrorId::ObjectUnavailable
    );
}

#[cfg(unix)]
#[test]
fn runtime_plan_rejects_installed_archive_same_byte_replaced_after_plan() {
    let (fixture, _package, _installed, _executable, plan) =
        runtime_plan("runtime-install-same-byte-after-plan", valid_args());
    replace_with_same_bytes(
        &fixture.root.join("plugins/harness-ultragoal.hugpkg"),
        "same-byte-install-after-plan",
    );
    assert_eq!(
        plan.execute_bound().unwrap_err().id(),
        ErrorId::ObjectChanged
    );
}

#[cfg(unix)]
#[test]
fn runtime_plan_rejects_installed_archive_mutate_restore_after_plan() {
    let (fixture, _package, _installed, _executable, plan) =
        runtime_plan("runtime-install-mutate-restore-after-plan", valid_args());
    let installed_path = fixture.root.join("plugins/harness-ultragoal.hugpkg");
    let original = std::fs::read(&installed_path).unwrap();
    std::fs::write(&installed_path, b"temporary runtime install mutation").unwrap();
    std::fs::write(&installed_path, original).unwrap();
    assert_eq!(
        plan.execute_bound().unwrap_err().id(),
        ErrorId::ObjectChanged
    );
}

#[cfg(unix)]
#[test]
fn runtime_plan_rejects_installed_archive_replaced_during_execution() {
    let (fixture, _package, _installed, _executable, plan) =
        runtime_plan("runtime-install-replaced-during-exec", slow_args());
    let installed_path = fixture.root.join("plugins/harness-ultragoal.hugpkg");
    let race = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        replace_with_same_bytes(&installed_path, "same-byte-install-during-exec");
    });
    let result = plan.execute_bound();
    race.join().unwrap();
    assert_eq!(result.unwrap_err().id(), ErrorId::ObjectChanged);
}

#[cfg(unix)]
#[test]
fn cloned_runtime_plan_rejects_replay_after_uninstall_or_reinstall() {
    let (fixture, package, installed, _executable, plan) =
        runtime_plan("runtime-install-clone-replay", valid_args());
    let replay = plan.clone();
    uninstall(
        "plugins/harness-ultragoal.hugpkg",
        installed.snapshot(),
        &mut ScopedInstall::new(fixture.confined()),
    )
    .unwrap();
    assert_eq!(
        replay.clone().execute_bound().unwrap_err().id(),
        ErrorId::ObjectUnavailable
    );

    let reinstall_plan = InstallPlan::new(
        package.context_id().into(),
        package.candidate_id().into(),
        InstallScope::PersonalFixture,
        "plugins/harness-ultragoal.hugpkg".into(),
        package.package_sha256().into(),
        ExpectedPrior::Absent,
    )
    .unwrap();
    install(
        &reinstall_plan,
        &package,
        &mut ScopedInstall::new(fixture.confined()),
    )
    .unwrap();
    assert_eq!(
        replay.execute_bound().unwrap_err().id(),
        ErrorId::ObjectChanged
    );
}
