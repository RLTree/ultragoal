use super::*;

pub(crate) static NEXT_SOURCE_FIXTURE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct SourceFixture {
    pub(crate) root: PathBuf,
    pub(crate) templates: PathBuf,
    pub(crate) output: PathBuf,
}

impl SourceFixture {
    pub(crate) fn new(label: &str) -> Self {
        let nonce = NEXT_SOURCE_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = PathBuf::from("/tmp/hul-repository-fit-public-adapter-056").join(format!(
            "build-source-{}-{}-{nonce}",
            label,
            std::process::id()
        ));
        let templates = root.join("templates");
        let output = root.join("out");
        fs::create_dir_all(&templates).unwrap();
        fs::create_dir_all(&output).unwrap();
        Self {
            root,
            templates,
            output,
        }
    }

    pub(crate) fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.templates.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    }

    pub(crate) fn staging(&self) -> PathBuf {
        self.output.join("repository_fit_template_sources")
    }

    pub(crate) fn catalog(&self) -> PathBuf {
        self.output.join("repository_fit_template_catalog.rs")
    }

    pub(crate) fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    pub(crate) fn write_manifest(&self, bytes: &[u8]) {
        let path = self.manifest_path();
        fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    }

    pub(crate) fn prepopulate_authority_outputs(&self) {
        fs::create_dir_all(self.staging()).unwrap();
        fs::write(self.staging().join("0000.bin"), b"stale staged bytes").unwrap();
        fs::write(self.catalog(), b"stale generated catalog").unwrap();
    }

    pub(crate) fn assert_authority_outputs_absent(&self) {
        assert!(!self.staging().exists(), "stale staging survived");
        assert!(!self.catalog().exists(), "stale catalog survived");
    }
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(crate) const A: TemplateCatalogRow = TemplateCatalogRow {
    source_path: "templates/A.md",
    target_path: "A.md",
    bytes: b"A\n",
    unix_mode: 0o644,
    source_kind: TemplateSourceKind::Regular,
};

pub(crate) fn manifest(paths: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&json!({"authorable_templates": paths})).unwrap()
}

pub(crate) fn assert_post_inspection_mutation_rejected<F>(label: &str, hook: F)
where
    F: FnOnce(&SourceFixture),
{
    let fixture = SourceFixture::new(label);
    fixture.write("A.md", b"prior");
    let result = production_sources::stage_manifest_sources_with_hook(
        &manifest(&["templates/A.md"]),
        &fixture.templates,
        &fixture.output,
        || hook(&fixture),
    );
    assert!(result.is_err(), "{label} unexpectedly passed");
    assert!(
        !fixture.staging().exists(),
        "{label} left failed staged output"
    );
    assert!(!fixture.catalog().exists(), "{label} left failed catalog");
}

pub(crate) fn rejected(
    context: &LiveContext,
    rows: &[TemplateCatalogRow],
    manifest: &[u8],
) -> AdapterErrorId {
    compile_rows(context, rows, manifest).err().unwrap().id()
}

#[test]
pub(crate) fn canonical_compile_time_catalog_matches_manifest_and_is_deterministic() {
    let fixture = Fixture::new("catalog-canonical");
    let context = fixture.context();
    let (first, second) = assert_zero_write(&fixture, || {
        (compile(&context).unwrap(), compile(&context).unwrap())
    });
    assert_eq!(CANONICAL_TEMPLATES.len(), 68);
    assert_eq!(first.authority, second.authority);
    assert_eq!(first.desired.state_sha256(), second.desired.state_sha256());
    assert_eq!(first.authority.template_count, 68);
    assert_eq!(first.unix_modes.len(), 68);
    assert!(first.authority.total_bytes > 900_000);
}

#[test]
pub(crate) fn catalog_authority_ignores_host_configuration_but_desired_state_binds_context() {
    let fixture = Fixture::new("catalog-config");
    let plain = fixture.context();
    let configured = LiveContext::build(
        BuildRequest::new(&fixture.root).bind_non_secret_configuration("profile", "test-value"),
    )
    .unwrap();
    let plain_bundle = compile(&plain).unwrap();
    let configured_bundle = compile(&configured).unwrap();
    assert_eq!(plain_bundle.authority, configured_bundle.authority);
    assert_ne!(plain.context_id(), configured.context_id());
    assert_ne!(
        plain_bundle.desired.state_sha256(),
        configured_bundle.desired.state_sha256()
    );
}

#[test]
pub(crate) fn duplicate_exact_and_ascii_case_alias_rows_are_rejected() {
    let fixture = Fixture::new("catalog-duplicate");
    let context = fixture.context();
    let duplicate = [A, A];
    assert_eq!(
        rejected(
            &context,
            &duplicate,
            &manifest(&["templates/A.md", "templates/B.md"])
        ),
        AdapterErrorId::InvalidTemplateCatalog
    );
    let alias = TemplateCatalogRow {
        source_path: "templates/a.md",
        target_path: "a.md",
        ..A
    };
    assert_eq!(
        rejected(
            &context,
            &[A, alias],
            &manifest(&["templates/A.md", "templates/a.md"])
        ),
        AdapterErrorId::InvalidTemplateCatalog
    );
}
