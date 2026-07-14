#[test]
fn source_symlink_and_hardlink_descriptors_are_rejected() {
    let repo = TempRepo::canonical();
    let path = repo.root.join(".codex/agents/repo-recon.toml");
    let outside = repo.root.join("outside.toml");
    fs::write(&outside, descriptor("repo-recon", "read-only")).unwrap();
    fs::remove_file(&path).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &path).unwrap();
    #[cfg(unix)]
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );

    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        fs::write(&path, descriptor("repo-recon", "read-only")).unwrap();
        fs::hard_link(&path, repo.root.join("outside-hardlink.toml")).unwrap();
        assert_eq!(
            SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
                .unwrap_err()
                .id(),
            AgentDiscoveryErrorId::UnsafeFilesystemEntry
        );
    }
}

#[cfg(unix)]
#[test]
fn plugin_manifest_final_symlink_and_authority_ancestor_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;

    let repo = TempRepo::canonical();
    let manifest = repo.root.join(".codex-plugin/plugin.json");
    let outside = repo.root.join("outside-plugin.json");
    fs::write(&outside, br#"{"name":"outside"}"#).unwrap();
    fs::remove_file(&manifest).unwrap();
    symlink(&outside, &manifest).unwrap();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );

    let repo = TempRepo::canonical();
    let codex = repo.root.join(".codex");
    let codex_real = repo.root.join(".codex-real");
    fs::rename(&codex, &codex_real).unwrap();
    symlink(&codex_real, &codex).unwrap();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );

    let repo = TempRepo::canonical();
    let plugin = repo.root.join(".codex-plugin");
    let plugin_real = repo.root.join(".codex-plugin-real");
    fs::rename(&plugin, &plugin_real).unwrap();
    symlink(&plugin_real, &plugin).unwrap();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );
}

#[cfg(unix)]
#[test]
fn source_fifo_and_socket_entries_are_rejected_without_opening_them() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::net::UnixListener;

    let repo = TempRepo::canonical();
    let fifo = repo.root.join(".codex/agents/extra-fifo.toml");
    let c_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );
    fs::remove_file(&fifo).unwrap();
    let socket = repo.root.join(".codex/agents/extra-socket.toml");
    let _listener = UnixListener::bind(&socket).unwrap();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );
}

#[cfg(unix)]
#[test]
fn large_extra_agent_directory_refuses_stably_with_bounded_metadata_only_work_and_zero_writes() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let repo = TempRepo::canonical();
    let agents = repo.root.join(".codex/agents");
    for index in 0..2_048 {
        fs::write(agents.join(format!("extra-{index:04}.toml")), [b'x']).unwrap();
    }
    let fifo = agents.join("special-extra-fifo.toml");
    let c_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    let before = tree_snapshot(&repo.root);

    reset_test_io_counts();
    let first = SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
        .unwrap_err()
        .id();
    let first_counts = test_io_counts();
    assert!(matches!(
        first,
        AgentDiscoveryErrorId::InvalidSourceCatalog | AgentDiscoveryErrorId::UnsafeFilesystemEntry
    ));
    assert!(first_counts.directory_entries <= 7);
    assert_eq!(first_counts.metadata_probes, first_counts.directory_entries);
    assert_eq!(first_counts.file_open_attempts, 0);
    assert_eq!(tree_snapshot(&repo.root), before);

    reset_test_io_counts();
    let second = SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
        .unwrap_err()
        .id();
    let second_counts = test_io_counts();
    assert_eq!(second, first);
    assert_eq!(second_counts, first_counts);
    assert_eq!(tree_snapshot(&repo.root), before);
}

#[cfg(unix)]
#[test]
fn plugin_metadata_requires_one_exact_regular_entry_before_any_source_body_read() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let repo = TempRepo::canonical();
    let fifo = repo.root.join(".codex-plugin/extra-fifo");
    let c_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    let before = tree_snapshot(&repo.root);
    reset_test_io_counts();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );
    let counts = test_io_counts();
    assert!(counts.directory_entries <= 8);
    assert_eq!(counts.metadata_probes, counts.directory_entries);
    assert_eq!(counts.file_open_attempts, 0);
    assert_eq!(tree_snapshot(&repo.root), before);
}
