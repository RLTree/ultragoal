#[test]
fn process_backend_is_descriptor_bound_empty_environment_contained_and_darwin_closed() {
    let process = source("validator/src/distribution/host_effect/executor/process.rs");
    let model = source("validator/src/distribution/host_effect/executor/model.rs");
    for required in [
        "execveat(",
        "libc::AT_EMPTY_PATH",
        "fexecve(",
        "executable.file().as_raw_fd()",
        "let environment = [std::ptr::null",
        "libc::setpgid",
        "terminate_process_group",
        "libc::kill(-child",
        "policy.stdout_limit()",
        "policy.stderr_limit()",
        "policy.timeout()",
        "UnsupportedPlatform",
        "defense in depth and performs no fork, spawn, or write",
    ] {
        assert!(
            process.contains(required),
            "missing process token {required}"
        );
    }
    assert!(!process.contains("Command::new"));
    assert!(!process.contains("/bin/sh"));
    assert!(!process.contains("/usr/bin/env"));
    for required in [
        "EXACT_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024",
        "MAX_TIMEOUT_MS: u64 = 5 * 60 * 1000",
        "if !environment.is_empty()",
        "EnvironmentInjection",
        "inherited_environment: false",
        "environment_entries: 0",
    ] {
        assert!(model.contains(required), "missing model token {required}");
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
