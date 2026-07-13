use super::support::{Fixture, assert_zero_write};
use crate::context::{BuildRequest, LiveContext};
use crate::repository_fit::product_adapter::AdapterErrorId;
use crate::repository_fit::product_adapter::catalog::{
    CANONICAL_TEMPLATES, TemplateCatalogRow, TemplateSourceKind, compile, compile_rows,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[path = "../../../../build_support/repository_fit_template_sources.rs"]
mod production_sources;

static NEXT_SOURCE_FIXTURE: AtomicU64 = AtomicU64::new(1);

struct SourceFixture {
    root: PathBuf,
    templates: PathBuf,
    output: PathBuf,
}

impl SourceFixture {
    fn new(label: &str) -> Self {
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

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.templates.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    }

    fn staging(&self) -> PathBuf {
        self.output.join("repository_fit_template_sources")
    }

    fn catalog(&self) -> PathBuf {
        self.output.join("repository_fit_template_catalog.rs")
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    fn write_manifest(&self, bytes: &[u8]) {
        let path = self.manifest_path();
        fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    }

    fn prepopulate_authority_outputs(&self) {
        fs::create_dir_all(self.staging()).unwrap();
        fs::write(self.staging().join("0000.bin"), b"stale staged bytes").unwrap();
        fs::write(self.catalog(), b"stale generated catalog").unwrap();
    }

    fn assert_authority_outputs_absent(&self) {
        assert!(!self.staging().exists(), "stale staging survived");
        assert!(!self.catalog().exists(), "stale catalog survived");
    }
}

impl Drop for SourceFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const A: TemplateCatalogRow = TemplateCatalogRow {
    source_path: "templates/A.md",
    target_path: "A.md",
    bytes: b"A\n",
    unix_mode: 0o644,
    source_kind: TemplateSourceKind::Regular,
};

fn manifest(paths: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&json!({"authorable_templates": paths})).unwrap()
}

fn assert_post_inspection_mutation_rejected<F>(label: &str, hook: F)
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

fn rejected(context: &LiveContext, rows: &[TemplateCatalogRow], manifest: &[u8]) -> AdapterErrorId {
    compile_rows(context, rows, manifest).err().unwrap().id()
}

#[test]
fn canonical_compile_time_catalog_matches_manifest_and_is_deterministic() {
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
fn catalog_authority_ignores_host_configuration_but_desired_state_binds_context() {
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
fn duplicate_exact_and_ascii_case_alias_rows_are_rejected() {
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

#[test]
fn traversal_absolute_and_source_target_mismatch_are_rejected() {
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
fn every_special_or_aliased_source_kind_is_rejected_without_opening_it() {
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
fn missing_unknown_and_reordered_manifest_rows_are_rejected() {
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
fn manifest_authority_drift_changes_identity_and_stales_prior_desired_state() {
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
fn empty_oversized_mode_and_malformed_manifest_controls_fail_closed() {
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
fn production_source_classifier_accepts_and_stages_one_real_regular_file() {
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

#[cfg(unix)]
#[test]
fn production_source_classifier_rejects_real_links_fifo_socket_and_directory_without_hanging() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    let symlink_fixture = SourceFixture::new("symlink");
    symlink_fixture.write("real", b"bytes");
    symlink("real", symlink_fixture.templates.join("A.md")).unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &symlink_fixture.templates,
            &symlink_fixture.output,
        )
        .is_err()
    );

    let hardlink_fixture = SourceFixture::new("hardlink");
    hardlink_fixture.write("real", b"bytes");
    fs::hard_link(
        hardlink_fixture.templates.join("real"),
        hardlink_fixture.templates.join("A.md"),
    )
    .unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &hardlink_fixture.templates,
            &hardlink_fixture.output,
        )
        .is_err()
    );

    let fifo_fixture = SourceFixture::new("fifo");
    let fifo = fifo_fixture.templates.join("A.md");
    let fifo_c = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo_c.as_ptr(), 0o644) }, 0);
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &fifo_fixture.templates,
            &fifo_fixture.output,
        )
        .is_err()
    );

    let socket_fixture = SourceFixture::new("socket");
    let _listener =
        std::os::unix::net::UnixListener::bind(socket_fixture.templates.join("A.md")).unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &socket_fixture.templates,
            &socket_fixture.output,
        )
        .is_err()
    );

    let directory_fixture = SourceFixture::new("directory");
    fs::create_dir(directory_fixture.templates.join("A.md")).unwrap();
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &directory_fixture.templates,
            &directory_fixture.output,
        )
        .is_err()
    );
}

#[test]
fn production_source_classifier_rejects_unknown_duplicate_and_source_list_drift() {
    let fixture = SourceFixture::new("list-drift");
    fixture.write("A.md", b"A");
    for manifest_bytes in [
        manifest(&["templates/Unknown.md"]),
        manifest(&["templates/A.md", "templates/A.md"]),
    ] {
        assert!(
            production_sources::stage_manifest_sources(
                &manifest_bytes,
                &fixture.templates,
                &fixture.output,
            )
            .is_err()
        );
    }
    fixture.write("B.md", b"B");
    assert!(
        production_sources::stage_manifest_sources(
            &manifest(&["templates/A.md"]),
            &fixture.templates,
            &fixture.output,
        )
        .is_err()
    );
}

#[test]
fn production_source_classifier_rejects_mutation_after_inspection_before_staging() {
    let fixture = SourceFixture::new("inspection-race");
    fixture.write("A.md", b"prior");
    let target = fixture.templates.join("A.md");
    let replacement = fixture.templates.join("replacement");
    let result = production_sources::stage_manifest_sources_with_hook(
        &manifest(&["templates/A.md"]),
        &fixture.templates,
        &fixture.output,
        || {
            fs::write(&replacement, b"later").unwrap();
            fs::rename(&replacement, &target).unwrap();
        },
    );
    assert!(result.is_err());
    assert!(!fixture.staging().join("0000.bin").exists());
}

#[test]
fn production_source_classifier_rejects_added_source_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-add", |fixture| {
        fixture.write("B.md", b"extra");
    });
}

#[test]
fn production_source_classifier_rejects_removed_source_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-remove", |fixture| {
        fs::remove_file(fixture.templates.join("A.md")).unwrap();
    });
}

#[test]
fn production_source_classifier_rejects_renamed_source_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-rename", |fixture| {
        fs::rename(
            fixture.templates.join("A.md"),
            fixture.templates.join("B.md"),
        )
        .unwrap();
    });
}

#[test]
fn production_source_classifier_rejects_casefold_extra_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-casefold", |fixture| {
        fixture.write("a.md", b"case alias");
    });
}

#[test]
fn production_source_classifier_rejects_nested_extra_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-nested", |fixture| {
        fixture.write("unrelated/B.md", b"nested extra");
    });
}

#[test]
fn production_source_classifier_rejects_directory_insertion_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-directory", |fixture| {
        fs::create_dir(fixture.templates.join("unrelated")).unwrap();
    });
}

#[test]
fn production_source_classifier_rejects_add_then_remove_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-add-remove", |fixture| {
        let transient = fixture.templates.join("B.md");
        fs::write(&transient, b"transient").unwrap();
        fs::remove_file(transient).unwrap();
    });
}

#[test]
fn production_source_classifier_rejects_mutate_then_restore_after_inspection() {
    assert_post_inspection_mutation_rejected("post-inspection-mutate-restore", |fixture| {
        let source = fixture.templates.join("A.md");
        fs::write(&source, b"changed").unwrap();
        fs::write(source, b"prior").unwrap();
    });
}

#[cfg(unix)]
#[test]
fn production_generate_erases_preexisting_outputs_before_manifest_validation() {
    use std::os::unix::fs::symlink;

    let fixture = SourceFixture::new("generate-manifest-early-failure");
    fixture.write("A.md", b"trusted");
    let real_manifest = fixture.root.join("real-manifest.json");
    fs::write(&real_manifest, manifest(&["templates/A.md"])).unwrap();
    symlink(&real_manifest, fixture.manifest_path()).unwrap();
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[cfg(unix)]
#[test]
fn production_generate_erases_preexisting_outputs_on_real_source_symlink_failure() {
    use std::os::unix::fs::symlink;

    let fixture = SourceFixture::new("generate-source-symlink-failure");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fs::write(fixture.root.join("real-source"), b"trusted").unwrap();
    symlink("../real-source", fixture.templates.join("A.md")).unwrap();
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
fn production_generate_erases_preexisting_outputs_on_parse_and_initial_parity_failures() {
    let malformed = SourceFixture::new("generate-parse-failure");
    malformed.write("A.md", b"trusted");
    malformed.write_manifest(b"not-json");
    malformed.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(
            &malformed.manifest_path(),
            &malformed.templates,
            &malformed.output,
        )
        .is_err()
    );
    malformed.assert_authority_outputs_absent();

    let parity = SourceFixture::new("generate-initial-parity-failure");
    parity.write("A.md", b"trusted");
    parity.write("B.md", b"extra");
    parity.write_manifest(&manifest(&["templates/A.md"]));
    parity.prepopulate_authority_outputs();
    assert!(
        production_sources::generate(&parity.manifest_path(), &parity.templates, &parity.output,)
            .is_err()
    );
    parity.assert_authority_outputs_absent();
}

#[test]
fn production_generate_erases_outputs_on_final_reenumeration_failure() {
    let fixture = SourceFixture::new("generate-final-parity-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate_with_hooks(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
            || fixture.write("B.md", b"extra"),
            || {},
            || {},
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
fn production_generate_catches_hook_panic_and_erases_outputs() {
    let fixture = SourceFixture::new("generate-hook-panic");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    let result = production_sources::generate_with_hooks(
        &fixture.manifest_path(),
        &fixture.templates,
        &fixture.output,
        || panic!("controlled build-source hook panic"),
        || {},
        || {},
    );
    assert_eq!(result.unwrap_err(), "template source staging panicked");
    fixture.assert_authority_outputs_absent();
}

#[test]
fn production_generate_erases_outputs_on_staged_manifest_write_failure() {
    let fixture = SourceFixture::new("generate-manifest-write-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate_with_hooks(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
            || {},
            || fs::create_dir(fixture.staging().join("manifest.json")).unwrap(),
            || {},
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
fn production_generate_erases_outputs_on_catalog_emission_failure() {
    let fixture = SourceFixture::new("generate-catalog-emission-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(&manifest(&["templates/A.md"]));
    fixture.prepopulate_authority_outputs();
    assert!(
        production_sources::generate_with_hooks(
            &fixture.manifest_path(),
            &fixture.templates,
            &fixture.output,
            || {},
            || {},
            || fs::create_dir(fixture.catalog()).unwrap(),
        )
        .is_err()
    );
    fixture.assert_authority_outputs_absent();
}

#[test]
fn production_generate_success_replaces_stale_outputs_with_exact_new_outputs() {
    let fixture = SourceFixture::new("generate-success-exact-outputs");
    fixture.write("A.md", b"trusted");
    let manifest_bytes = manifest(&["templates/A.md"]);
    fixture.write_manifest(&manifest_bytes);
    fixture.prepopulate_authority_outputs();
    production_sources::generate(
        &fixture.manifest_path(),
        &fixture.templates,
        &fixture.output,
    )
    .unwrap();
    assert_eq!(
        fs::read(fixture.staging().join("0000.bin")).unwrap(),
        b"trusted"
    );
    assert_eq!(
        fs::read(fixture.staging().join("manifest.json")).unwrap(),
        manifest_bytes
    );
    let mut staged_names = fs::read_dir(fixture.staging())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    staged_names.sort();
    assert_eq!(staged_names, ["0000.bin", "manifest.json"]);
    let catalog = fs::read_to_string(fixture.catalog()).unwrap();
    assert!(catalog.starts_with("const MANIFEST_BYTES"));
    assert!(!catalog.contains("stale generated catalog"));
}

#[cfg(unix)]
#[test]
fn production_generate_reports_cleanup_failure_before_validation() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = SourceFixture::new("generate-cleanup-failure");
    fixture.write("A.md", b"trusted");
    fixture.write_manifest(b"not-json");
    fixture.prepopulate_authority_outputs();
    fs::set_permissions(&fixture.output, fs::Permissions::from_mode(0o555)).unwrap();
    let result = production_sources::generate(
        &fixture.manifest_path(),
        &fixture.templates,
        &fixture.output,
    );
    fs::set_permissions(&fixture.output, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        result.unwrap_err(),
        "authority-bearing output cleanup failed"
    );
    assert!(fixture.catalog().exists() || fixture.staging().exists());
}
