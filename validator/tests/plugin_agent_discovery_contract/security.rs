use crate::agent_discovery::{
    AgentAuthorityLayer, AgentDiscoveryErrorId, AgentDiscoverySession, SourceAgentCatalog,
    reset_test_io_counts, set_test_readdir_fault, test_io_counts, test_readdir_fault_triggered,
};
use crate::support::{CANDIDATE, FixtureReader, SESSION, TempRepo, descriptor, tree_snapshot};
use std::fs;

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

#[cfg(unix)]
#[test]
fn unicode_and_oversized_unexpected_names_fail_before_source_body_reads() {
    let repo = TempRepo::canonical();
    repo.write(".codex/agents/éxtra.toml", b"untrusted");
    reset_test_io_counts();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::InvalidSourceCatalog
    );
    let unicode_counts = test_io_counts();
    assert!(unicode_counts.directory_entries <= 7);
    assert_eq!(unicode_counts.file_open_attempts, 0);

    let repo = TempRepo::canonical();
    let oversized = format!("{}.toml", "x".repeat(120));
    repo.write(&format!(".codex/agents/{oversized}"), b"untrusted");
    reset_test_io_counts();
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::InputTooLarge
    );
    let oversized_counts = test_io_counts();
    assert!(oversized_counts.directory_entries <= 7);
    assert_eq!(oversized_counts.file_open_attempts, 0);
}

#[cfg(unix)]
fn assert_readdir_fault_fails_closed_stably(
    scan_ordinal: usize,
    after_entries: usize,
    error_number: libc::c_int,
) {
    let repo = TempRepo::canonical();
    let before = tree_snapshot(&repo.root);
    let mut prior = None;
    for _ in 0..2 {
        reset_test_io_counts();
        set_test_readdir_fault(scan_ordinal, after_entries, error_number, true);
        let error = SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id();
        let counts = test_io_counts();
        assert_eq!(error, AgentDiscoveryErrorId::UnsafeFilesystemEntry);
        assert!(test_readdir_fault_triggered());
        assert_eq!(counts.readdir_errors, 1);
        assert_eq!(counts.failed_stream_closes, 1);
        assert_eq!(counts.post_terminal_calls, 1);
        assert_eq!(counts.file_open_attempts, 0);
        assert_eq!(tree_snapshot(&repo.root), before);
        if let Some((prior_error, prior_counts)) = prior {
            assert_eq!(error, prior_error);
            assert_eq!(counts, prior_counts);
        }
        prior = Some((error, counts));
    }
}

#[cfg(unix)]
#[test]
fn agent_readdir_eio_after_six_expected_rows_cannot_false_pass_as_eof() {
    assert_readdir_fault_fails_closed_stably(0, 6, libc::EIO);
}

#[cfg(unix)]
#[test]
fn plugin_readdir_eio_after_required_row_cannot_false_pass_as_eof() {
    assert_readdir_fault_fails_closed_stably(1, 1, libc::EIO);
}

#[cfg(unix)]
#[test]
fn readdir_eintr_is_not_retried_and_failed_stream_is_terminal() {
    assert_readdir_fault_fails_closed_stably(0, 6, libc::EINTR);
}

#[test]
fn oversized_and_duplicate_key_source_descriptors_fail_closed() {
    let repo = TempRepo::canonical();
    repo.write(".codex/agents/repo-recon.toml", vec![b'x'; 64 * 1024 + 1]);
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::InputTooLarge
    );

    let repo = TempRepo::canonical();
    repo.write(
        ".codex/agents/repo-recon.toml",
        format!(
            "{}name = \"repo-recon\"\n",
            descriptor("repo-recon", "read-only")
        ),
    );
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::InvalidSourceCatalog
    );
}

fn verify_host_mutation(
    configure: impl FnOnce(&mut crate::support::FixtureTransaction) + 'static,
) -> AgentDiscoveryErrorId {
    let repo = TempRepo::canonical();
    let source = repo.capture();
    let session = AgentDiscoverySession::bind(source.clone()).unwrap();
    let mut reader = FixtureReader::exact(&source);
    reader.configure(configure);
    session.verify(&mut reader).unwrap_err().id()
}

#[test]
fn host_special_file_hardlink_path_escape_and_oversize_rows_fail_closed() {
    for kind in ["symlink", "fifo", "socket", "device"] {
        assert_eq!(
            verify_host_mutation(move |transaction| {
                transaction.mutate_catalog(AgentAuthorityLayer::Package, |catalog| {
                    catalog["agents"][0]["file_kind"] = serde_json::json!(kind);
                });
            }),
            AgentDiscoveryErrorId::UnsafeFilesystemEntry
        );
    }
    assert_eq!(
        verify_host_mutation(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Installed, |catalog| {
                catalog["agents"][0]["link_count"] = serde_json::json!(2);
            });
        }),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );
    assert_eq!(
        verify_host_mutation(|transaction| {
            transaction.mutate_catalog(AgentAuthorityLayer::Cache, |catalog| {
                catalog["agents"][0]["manifest_path"] = serde_json::json!("../../escape.toml");
            });
        }),
        AgentDiscoveryErrorId::UnsafeFilesystemEntry
    );
    assert_eq!(
        verify_host_mutation(|transaction| {
            transaction.catalogs.insert(
                AgentAuthorityLayer::Discovery,
                vec![b'x'; 2 * 1024 * 1024 + 1],
            );
        }),
        AgentDiscoveryErrorId::InputTooLarge
    );
}

#[test]
fn duplicate_json_fields_and_non_utf8_toml_are_rejected() {
    assert_eq!(
        verify_host_mutation(|transaction| {
            let bytes = transaction
                .catalogs
                .get(&AgentAuthorityLayer::Package)
                .unwrap();
            let text = std::str::from_utf8(bytes).unwrap();
            let duplicate = text.replacen(
                "\"schema_version\":",
                "\"schema_version\":\"HostAgentAuthorityCatalog-v1\",\"schema_version\":",
                1,
            );
            transaction
                .catalogs
                .insert(AgentAuthorityLayer::Package, duplicate.into_bytes());
        }),
        AgentDiscoveryErrorId::ObservationConflict
    );
    let repo = TempRepo::canonical();
    repo.write(".codex/agents/repo-recon.toml", [0xff, 0xfe]);
    assert_eq!(
        SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
            .unwrap_err()
            .id(),
        AgentDiscoveryErrorId::InvalidSourceCatalog
    );
}

#[test]
fn errors_do_not_echo_host_or_descriptor_input() {
    let secret = "SECRET-CANARY-DO-NOT-ECHO";
    let error = verify_host_mutation(move |transaction| {
        transaction.mutate_catalog(AgentAuthorityLayer::Global, |catalog| {
            catalog[secret] = serde_json::json!(secret);
        });
    });
    let rendered = format!(
        "{}",
        crate::agent_discovery::AgentDiscoveryError::new(error)
    );
    assert!(!rendered.contains(secret));
}
