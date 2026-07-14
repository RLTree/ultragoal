use super::*;

#[test]
pub(crate) fn traversal_absolute_and_source_target_mismatch_are_rejected() {
    let fixture = Fixture::new("catalog-paths");
    let context = fixture.context();
    for row in [
        TemplateCatalogRow {
            source_path: "templates/../A.md",
            target_path: "../A.md",
            ..A
        },
        TemplateCatalogRow {
            source_path: "/templates/A.md",
            ..A
        },
        TemplateCatalogRow {
            source_path: "templates/B.md",
            ..A
        },
    ] {
        assert_eq!(
            rejected(&context, &[row], &manifest(&["templates/A.md"])),
            AdapterErrorId::InvalidTemplateCatalog
        );
    }
}

#[test]
pub(crate) fn every_special_or_aliased_source_kind_is_rejected_without_opening_it() {
    let fixture = Fixture::new("catalog-special");
    let context = fixture.context();
    for source_kind in [
        TemplateSourceKind::Symlink,
        TemplateSourceKind::Hardlink,
        TemplateSourceKind::Directory,
        TemplateSourceKind::Fifo,
        TemplateSourceKind::Socket,
        TemplateSourceKind::CrossDevice,
    ] {
        let row = TemplateCatalogRow { source_kind, ..A };
        assert_eq!(
            rejected(&context, &[row], &manifest(&["templates/A.md"])),
            AdapterErrorId::InvalidTemplateCatalog
        );
    }
}

#[test]
pub(crate) fn missing_unknown_and_reordered_manifest_rows_are_rejected() {
    let fixture = Fixture::new("catalog-manifest");
    let context = fixture.context();
    let b = TemplateCatalogRow {
        source_path: "templates/B.md",
        target_path: "B.md",
        bytes: b"B\n",
        ..A
    };
    for manifest_bytes in [
        manifest(&["templates/A.md", "templates/B.md"]),
        manifest(&["templates/Unknown.md"]),
        manifest(&["templates/B.md", "templates/A.md"]),
        manifest(&["templates/A.md", "templates/A.md"]),
        manifest(&["templates/a.md"]),
        serde_json::to_vec(&json!({"not_authorable_templates": []})).unwrap(),
    ] {
        assert_eq!(
            rejected(&context, &[A], &manifest_bytes),
            AdapterErrorId::InvalidTemplateCatalog
        );
    }
    assert_eq!(
        rejected(&context, &[A, b], &manifest(&["templates/A.md"])),
        AdapterErrorId::InvalidTemplateCatalog
    );
}

#[test]
pub(crate) fn manifest_authority_drift_changes_identity_and_stales_prior_desired_state() {
    let fixture = Fixture::new("catalog-drift");
    let context = fixture.context();
    let first = compile_rows(
        &context,
        &[A],
        &serde_json::to_vec(&json!({
            "authorable_templates": ["templates/A.md"],
            "version": 1
        }))
        .unwrap(),
    )
    .unwrap();
    let second = compile_rows(
        &context,
        &[A],
        &serde_json::to_vec(&json!({
            "authorable_templates": ["templates/A.md"],
            "version": 2
        }))
        .unwrap(),
    )
    .unwrap();
    assert_ne!(
        first.authority.manifest_sha256,
        second.authority.manifest_sha256
    );
    assert_ne!(
        first.authority.authority_sha256,
        second.authority.authority_sha256
    );
    assert_ne!(first.desired.state_sha256(), second.desired.state_sha256());
}

#[test]
pub(crate) fn empty_oversized_mode_and_malformed_manifest_controls_fail_closed() {
    let fixture = Fixture::new("catalog-invalid");
    let context = fixture.context();
    assert_eq!(
        rejected(&context, &[], &manifest(&[])),
        AdapterErrorId::InvalidTemplateCatalog
    );
    let bad_mode = TemplateCatalogRow {
        unix_mode: 0o4755,
        ..A
    };
    assert_eq!(
        rejected(&context, &[bad_mode], &manifest(&["templates/A.md"])),
        AdapterErrorId::InvalidTemplateCatalog
    );
    assert_eq!(
        rejected(&context, &[A], b"not-json"),
        AdapterErrorId::InvalidTemplateCatalog
    );
}

#[test]
pub(crate) fn production_source_classifier_accepts_and_stages_one_real_regular_file() {
    let fixture = SourceFixture::new("regular");
    fixture.write("A.md", b"trusted bytes\n");
    let rows = production_sources::stage_manifest_sources(
        &manifest(&["templates/A.md"]),
        &fixture.templates,
        &fixture.output,
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source_path, "templates/A.md");
    assert_eq!(rows[0].target_path, "A.md");
    assert_eq!(rows[0].bytes, b"trusted bytes\n");
    assert_eq!(rows[0].unix_mode, 0o644);
    assert_eq!(
        fs::read(
            fixture
                .output
                .join("repository_fit_template_sources/0000.bin")
        )
        .unwrap(),
        b"trusted bytes\n"
    );
}
