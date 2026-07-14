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
    fault: ReaddirTestFault,
) {
    let repo = TempRepo::canonical();
    let before = tree_snapshot(&repo.root);
    let mut prior = None;
    for _ in 0..2 {
        reset_test_io_counts();
        set_test_readdir_fault(scan_ordinal, after_entries, fault, true);
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
    assert_readdir_fault_fails_closed_stably(0, 6, ReaddirTestFault::Io);
}

#[cfg(unix)]
#[test]
fn plugin_readdir_eio_after_required_row_cannot_false_pass_as_eof() {
    assert_readdir_fault_fails_closed_stably(1, 1, ReaddirTestFault::Io);
}

#[cfg(unix)]
#[test]
fn readdir_eintr_is_not_retried_and_failed_stream_is_terminal() {
    assert_readdir_fault_fails_closed_stably(0, 6, ReaddirTestFault::Interrupted);
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
    configure: impl FnOnce(&mut crate::authority_fixtures::FixtureTransaction) + 'static,
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
