#[test]
fn process_backend_is_descriptor_bound_empty_environment_contained_and_darwin_closed() {
    let executor = source("validator/src/distribution/host_effect/executor/mod.rs");
    let descriptor = source(
        "validator/src/distribution/host_effect/selected_codex_executable/execution/descriptor.rs",
    );
    let process_group = source(
        "validator/src/distribution/host_effect/selected_codex_executable/execution/process_group.rs",
    );
    let launch = source(
        "validator/src/distribution/host_effect/selected_codex_executable/immutable_launch.rs",
    );
    let darwin = source(
        "validator/src/distribution/host_effect/selected_codex_executable/execution/darwin.rs",
    );
    let process_custody = source("validator/src/process_custody/mod.rs");
    let routine_darwin_spawn =
        source("validator/src/routine_work/runtime_adapter/mediator/process/darwin_spawn.rs");
    let exact_output_limit =
        source("validator/src/distribution/host_effect/executor/model/exact_output_limit_bytes.rs");
    let policy =
        source("validator/src/distribution/host_effect/executor/model/execution_policy/strict.rs");
    for required in [
        "execveat(",
        "libc::AT_EMPTY_PATH",
        "fexecve(",
        "executable.launch_file().as_raw_fd()",
        "let environment_values",
        "environment.push(std::ptr::null())",
        "libc::fchdir(cwd)",
        "policy.stdout_limit()",
        "policy.stderr_limit()",
        "policy.timeout()",
    ] {
        assert!(
            descriptor.contains(required),
            "missing descriptor token {required}"
        );
    }
    for required in ["libc::setpgid", "terminate_process_group"] {
        assert!(
            descriptor.contains(required),
            "missing descriptor token {required}"
        );
    }
    for required in ["libc::kill(-child", "libc::kill(child"] {
        assert!(
            process_group.contains(required),
            "missing process-group token {required}"
        );
    }
    for required in [
        "libc::memfd_create",
        "libc::F_SEAL_SEAL",
        "libc::F_SEAL_WRITE",
        "libc::F_GET_SEALS",
        "expected_sha256",
    ] {
        assert!(launch.contains(required), "missing launch token {required}");
    }
    assert!(executor.contains("Darwin remains unsupported"));
    assert!(darwin.contains("UnsupportedPlatform"));
    assert!(darwin.contains("refuse before spawn"));
    assert!(!darwin.contains("posix_spawn"));
    assert!(!darwin.contains("process_custody"));
    assert!(!darwin.contains("program.path()"));
    assert!(!process_custody.contains("spawn_suspended_descriptor"));
    assert!(routine_darwin_spawn.contains("program: &PinnedExecutable"));
    assert!(!descriptor.contains("Command::new"));
    assert!(!descriptor.contains("/bin/sh"));
    assert!(!descriptor.contains("/usr/bin/env"));
    for required in [
        "EXACT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024",
        "MAX_TIMEOUT_MS: u64 = 5 * 60 * 1000",
    ] {
        assert!(
            exact_output_limit.contains(required),
            "missing output-limit token {required}"
        );
    }
    for required in [
        "if !environment.is_empty()",
        "EnvironmentInjection",
        "inherited_environment: false",
        "environment_entries: environment.len()",
    ] {
        assert!(policy.contains(required), "missing policy token {required}");
    }
}

#[test]
fn target_publication_is_descriptor_relative_exclusive_synced_and_exactly_reobserved() {
    let target = source("validator/src/distribution/host_effect/executor/target.rs");
    let normalized = target.split_whitespace().collect::<Vec<_>>().join(" ");
    for required in [
        "libc::openat(",
        "libc::O_NOFOLLOW",
        "libc::O_EXCL",
        "file.write_all(&prepared.bytes)",
        "libc::fchmod",
        "file.sync_all()",
        "rename_noreplace(",
        "libc::renameatx_np(",
        "libc::RENAME_EXCL",
        "RENAME_NOFOLLOW_ANY",
        "RENAME_RESOLVE_BENEATH",
        "BeforeDirectoryFsync",
        "CommittedBeforeAcknowledgement",
        "initially_created.links != 1",
        "before.links != 1",
        "PublicationObjectKind::Symlink",
        "PublicationObjectKind::Fifo",
        "PublicationObjectKind::Socket",
        "PublicationObjectKind::Device",
    ] {
        assert!(target.contains(required), "missing target token {required}");
    }
    assert_before(
        &target,
        "file.write_all(&prepared.bytes)",
        "file.sync_all()",
    );
    assert_before(&target, "file.sync_all()", "rename_noreplace(");
    assert!(normalized.contains("self.anchor .directory .sync_all()"));
    assert_before(
        &normalized,
        "rename_noreplace(",
        "self.anchor .directory .sync_all()",
    );
    assert!(!target.contains("fs::rename("));
    assert!(!target.contains("File::create("));
}
