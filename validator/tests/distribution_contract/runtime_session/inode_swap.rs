#[cfg(unix)]
fn runtime_fixture(
    label: &str,
) -> (
    JourneyFixture,
    PackageSnapshot,
    InstallTransaction,
    PathBuf,
    HostCapabilityDeclaration,
    JourneyBinding,
) {
    let fixture = JourneyFixture::new(label);
    let package = fixture.build("packages/runtime.hugpkg");
    let mut installed = install_for_runtime(&fixture, &package);
    let executable = installed_program(&fixture.root);
    let host = HostCapabilityDeclaration::isolated(
        &fixture.root,
        &fixture.project,
        "isolated-host-v1",
        Some(&executable),
    )
    .unwrap();
    let binding =
        JourneyBinding::new(package.identity().clone(), &host, "local-harness-plugins").unwrap();
    installed.bind_journey(&binding).unwrap();
    (fixture, package, installed, executable, host, binding)
}

#[cfg(unix)]
fn replace_with_same_bytes(path: &std::path::Path, suffix: &str) {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let before = std::fs::symlink_metadata(path).unwrap();
    let replacement = path.with_file_name(format!(
        "{}-{suffix}",
        path.file_name().unwrap().to_string_lossy()
    ));
    let bytes = std::fs::read(path).unwrap();
    std::fs::write(&replacement, bytes).unwrap();
    std::fs::set_permissions(
        &replacement,
        std::fs::Permissions::from_mode(before.mode() & 0o777),
    )
    .unwrap();
    let replacement_meta = std::fs::symlink_metadata(&replacement).unwrap();
    assert_ne!(
        (before.dev(), before.ino()),
        (replacement_meta.dev(), replacement_meta.ino())
    );
    std::fs::rename(replacement, path).unwrap();
    let after = std::fs::symlink_metadata(path).unwrap();
    assert_ne!((before.dev(), before.ino()), (after.dev(), after.ino()));
    assert_eq!(before.len(), after.len());
    assert_eq!(before.mode() & 0o777, after.mode() & 0o777);
}

#[cfg(unix)]
#[test]
fn same_byte_inode_swap_before_spawn_never_launches_or_accepts() {
    let (fixture, package, installed, executable, host, binding) =
        runtime_fixture("runtime-same-byte-before-spawn");
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let plan = RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
        binding,
        host: &host,
        install: installed.snapshot(),
        effects: &mut install_effects,
        package: &package,
        program: &executable,
        timeout: Duration::from_secs(10),
    })
    .unwrap();

    replace_with_same_bytes(&executable, "same-bytes-before-spawn");

    assert_eq!(
        execute_runtime_probe(&plan).unwrap_err().id(),
        ErrorId::ObjectChanged
    );
}

#[cfg(unix)]
#[test]
fn same_byte_inode_swap_during_execution_is_not_accepted() {
    let (fixture, package, installed, executable, host, binding) =
        runtime_fixture("runtime-same-byte-during-exec");
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let plan = RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
        binding,
        host: &host,
        install: installed.snapshot(),
        effects: &mut install_effects,
        package: &package,
        program: &executable,
        timeout: Duration::from_secs(10),
    })
    .unwrap();
    let executable_for_thread = executable.clone();
    let race = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        replace_with_same_bytes(&executable_for_thread, "same-bytes-during-exec");
    });

    let result = execute_runtime_probe(&plan);
    race.join().unwrap();

    assert_eq!(result.unwrap_err().id(), ErrorId::ObjectChanged);
}

#[cfg(unix)]
#[test]
fn sibling_wrong_route_and_replaced_object_regressions_fail_closed() {
    let (fixture, package, installed, executable, host, binding) =
        runtime_fixture("runtime-wrong-route-regressions");
    let mut install_effects = ScopedInstall::new(fixture.confined());
    let sibling = fixture.root.join("runtime/sibling-ultragoal");
    std::fs::copy(&executable, &sibling).unwrap();
    assert_eq!(
        RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
            binding: binding.clone(),
            host: &host,
            install: installed.snapshot(),
            effects: &mut install_effects,
            package: &package,
            program: &sibling,
            timeout: Duration::from_secs(10),
        })
        .unwrap_err()
        .id(),
        ErrorId::CapabilityMismatch
    );

    replace_with_same_bytes(&executable, "same-bytes-before-plan");
    let mut replaced_install_effects = ScopedInstall::new(fixture.confined());

    assert_eq!(
        RuntimeProbePlan::from_installed_package(InstalledPackageRuntimeProbeRequest {
            binding,
            host: &host,
            install: installed.snapshot(),
            effects: &mut replaced_install_effects,
            package: &package,
            program: &executable,
            timeout: Duration::from_secs(10),
        })
        .unwrap_err()
        .id(),
        ErrorId::CapabilityMismatch
    );
}
