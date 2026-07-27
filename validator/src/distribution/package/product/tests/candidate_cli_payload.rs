#[test]
fn candidate_cli_is_an_exact_bound_archive_member_and_materializes() {
    let repo = Repo::new("candidate-cli-payload");
    let context = repo.context();
    let catalog = catalog(&context);
    let source_artifact = capture_product_package(&context, &catalog).expect("source package");
    let cli_bytes = test_cli_bytes();
    let payload = CandidateCliPayload::for_candidate(source_artifact.candidate_id(), cli_bytes)
        .expect("native candidate CLI");
    let artifact = capture_product_package_with_cli(&context, &catalog, payload.clone())
        .expect("candidate package");

    let entry = artifact
        .plan
        .entries
        .iter()
        .find(|entry| entry.path == CLI_RUNTIME_ENTRY)
        .expect("candidate CLI archive member");
    assert_eq!(entry.mode, 0o755);
    assert_eq!(entry.role, PackageRole::Executable);
    assert_eq!(entry.sha256, payload.sha256());
    assert_eq!(entry.bytes, payload.bytes());
    assert!(
        artifact
            .plan
            .entries
            .iter()
            .all(|entry| entry.path != "runtime/runtime-probe-bin"),
        "a candidate CLI package must contain only the dynamic runtime executable"
    );

    let output = OutputRoot::new("candidate-cli-payload");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let mut tree =
        ScopedTree::new(confined, "plugins/harness-ultragoal").expect("marketplace package tree");
    artifact
        .materialize_marketplace_source(&context, &catalog, &mut tree)
        .expect("materialized candidate package");
    let installed = output
        .root
        .join("plugins/harness-ultragoal/runtime/ultragoal");
    assert_eq!(
        fs::read(&installed).expect("installed CLI"),
        payload.bytes()
    );
    #[cfg(unix)]
    assert_eq!(
        fs::metadata(&installed)
            .expect("installed metadata")
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
}

#[test]
fn candidate_cli_payload_rejects_candidate_substitution() {
    let repo = Repo::new("candidate-cli-substitution");
    let context = repo.context();
    let catalog = catalog(&context);
    let source_artifact = capture_product_package(&context, &catalog).expect("source package");
    let cli_bytes = test_cli_bytes();
    let payload = CandidateCliPayload::for_candidate(
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        cli_bytes,
    )
    .expect("well-formed substituted payload");
    let error =
        capture_product_package_with_cli(&context, &catalog, payload).expect_err("mismatch");
    assert_eq!(error.id(), ProductionPackageErrorId::CatalogMismatch);
    assert_ne!(
        source_artifact.candidate_id(),
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    );
}

#[test]
fn candidate_cli_materialization_refuses_substituted_output_without_overwrite() {
    let repo = Repo::new("candidate-cli-recovery");
    let context = repo.context();
    let catalog = catalog(&context);
    let source_artifact = capture_product_package(&context, &catalog).expect("source package");
    let cli_bytes = test_cli_bytes();
    let payload = CandidateCliPayload::for_candidate(source_artifact.candidate_id(), cli_bytes)
        .expect("native candidate CLI");
    let artifact = capture_product_package_with_cli(&context, &catalog, payload.clone())
        .expect("candidate package");
    let output = OutputRoot::new("candidate-cli-recovery");
    let confined = ConfinedRoot::open(&output.root).expect("confined root");
    let mut tree =
        ScopedTree::new(confined, "plugins/harness-ultragoal").expect("marketplace package tree");
    artifact
        .materialize_marketplace_source(&context, &catalog, &mut tree)
        .expect("materialized candidate package");
    let installed = output
        .root
        .join("plugins/harness-ultragoal/runtime/ultragoal");
    fs::write(&installed, b"substituted").expect("substitute installed CLI");
    assert!(
        artifact
            .materialize_marketplace_source(&context, &catalog, &mut tree)
            .is_err()
    );
    assert_eq!(
        fs::read(installed).expect("retained substituted bytes"),
        b"substituted"
    );
}

fn test_cli_bytes() -> Vec<u8> {
    let mut bytes = vec![
        0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 2, 0, 0, 0, 96, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0,
    ];
    bytes.extend_from_slice(&[0x19, 0, 0, 0, 72, 0, 0, 0]);
    bytes.extend_from_slice(&[0; 64]);
    bytes.extend_from_slice(&[0x28, 0, 0, 0x80, 24, 0, 0, 0]);
    bytes.extend_from_slice(&[0; 16]);
    bytes
}
#[test]
fn compiled_payload_rejects_probe_scripts_and_empty_bytes() {
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    for bytes in [Vec::new(), b"#!/bin/sh\necho probe\n".to_vec()] {
        assert!(CandidateCliPayload::for_candidate(candidate, bytes).is_err());
    }
}

#[test]
fn compiled_payload_rejects_truncated_or_incoherent_native_headers() {
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    for bytes in [
        b"\x7fELFcli".to_vec(),
        b"MZnot-a-portable-executable".to_vec(),
        b"\xcf\xfa\xed\xfecandidate-cli".to_vec(),
        vec![
            0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ],
    ] {
        assert!(CandidateCliPayload::for_candidate(candidate, bytes).is_err());
    }
}

#[test]
fn compiled_payload_preserves_candidate_and_digest() {
    let candidate = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let payload =
        CandidateCliPayload::for_candidate(candidate, test_cli_bytes()).expect("native payload");
    assert_eq!(payload.candidate_id(), candidate);
    assert_eq!(payload.bytes(), test_cli_bytes());
    assert!(payload.sha256().starts_with("sha256:"));
}

#[test]
fn source_owned_cli_path_is_rejected_without_a_candidate_payload() {
    let repo = Repo::new("candidate-cli-source-fallback");
    let source_path = repo.root.join("runtime/ultragoal");
    fs::write(&source_path, b"#!/bin/sh\necho fallback\n").expect("source CLI fallback");
    #[cfg(unix)]
    fs::set_permissions(&source_path, fs::Permissions::from_mode(0o755)).expect("source CLI mode");
    let context = repo.context();
    assert!(capture_product_package(&context, &catalog(&context)).is_err());
}
