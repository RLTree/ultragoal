#[cfg(unix)]
#[test]
fn symlink_hardlink_fifo_and_socket_agent_entries_block_without_opening_specials() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;

    for mutation in ["symlink", "fifo"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        repo.commit();
        match mutation {
            "symlink" => symlink(
                "repo-recon.toml",
                repo.root.join(".codex/agents/extra-link.toml"),
            )
            .unwrap(),
            "fifo" => {
                let fifo = repo.root.join(".codex/agents/special-fifo");
                let name = CString::new(fifo.as_os_str().as_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
            }
            _ => unreachable!(),
        }
        assert_catalog_blocks_route(&repo);
    }

    let repo = TestRepo::new("agent-route-hardlink");
    prepare(&repo, &[CASES[0]], true);
    repo.commit();
    fs::hard_link(
        repo.root.join(".codex/agents/repo-recon.toml"),
        repo.root.join("outside-hardlink.toml"),
    )
    .unwrap();
    assert!(
        catalog_result(&repo)
            .unwrap_err()
            .to_string()
            .contains("hard links")
    );

    let repo = TestRepo::new("agent-route-socket");
    prepare(&repo, &[CASES[0]], true);
    repo.commit();
    let short = std::path::PathBuf::from(format!("/tmp/uga-socket-{}", std::process::id()));
    let _ = fs::remove_file(&short);
    symlink(repo.root.join(".codex/agents"), &short).unwrap();
    let listener = UnixListener::bind(short.join("s")).unwrap();
    assert_catalog_blocks_route(&repo);
    drop(listener);
    fs::remove_file(short).unwrap();
}

#[test]
fn changed_allowlist_or_generic_package_consumer_blocks_route() {
    for mutation in ["allowlist", "generic-consumer"] {
        let repo = TestRepo::new(&format!("agent-route-{mutation}"));
        prepare(&repo, &[CASES[0]], true);
        let path = match mutation {
            "allowlist" => "validator/src/inventory/agent_reader_guard_digests/mod.rs",
            _ => "validator/src/package/inventory/mod.rs",
        };
        let mut bytes = fs::read(repo.root.join(path)).unwrap();
        bytes.extend_from_slice(b"\n// drift\n");
        repo.write(path, &bytes);
        assert_reader_drift_blocks_route(&repo);
    }
}
