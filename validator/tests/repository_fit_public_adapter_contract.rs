use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn source(relative: &str) -> String {
    fs::read_to_string(repository_root().join(relative)).unwrap()
}

#[test]
fn fixture_catalog_covers_the_exact_representative_and_false_pass_matrix() {
    let value: Value = serde_json::from_str(&source(
        "fixtures/repository-fit-public-adapter/catalog.json",
    ))
    .unwrap();
    assert_eq!(
        value["schema_version"],
        "RepositoryFitPublicAdapterFixtureCatalog-v1"
    );
    assert_eq!(
        value["temporary_root"],
        "/tmp/hul-repository-fit-public-adapter-056"
    );
    let ids = value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    for required in [
        "fresh-repository",
        "partial-retrofit",
        "conflicting-authority",
        "dirty-repository",
        "already-fitted-repeat-use",
        "matching-bytes-mode-drift",
        "matching-bytes-setuid-drift",
        "matching-bytes-setgid-drift",
        "matching-bytes-sticky-drift",
        "mode-bound-verification-replay",
        "failed-apply-rollback",
        "rollback-ambiguity-recovery",
        "stale-context",
        "stale-target",
        "stale-plan",
        "stale-template",
        "stale-acceptance",
        "production-regular-template-source",
        "production-template-source-symlink",
        "production-template-source-hardlink",
        "production-template-source-fifo-socket-directory",
        "production-template-source-list-drift",
        "production-template-source-inspection-race",
        "target-symlink-hardlink",
        "target-fifo-socket",
        "leaf-path-swap",
        "post-swap-identity-race",
        "replacement-temp-cleanup-race",
        "removal-temp-cleanup-race",
        "staged-parent-publish-race",
        "created-parent-cleanup-race",
        "cross-device-object",
        "zero-hidden-write-read-paths",
        "opaque-accepted-plan",
        "missing-exclusive-mutation-lease",
    ] {
        assert!(ids.contains(&required), "missing fixture {required}");
    }
    assert_eq!(value["claim_effect"], "none");
}

#[test]
fn adapter_is_crate_private_and_opaque_without_effect_authority() {
    let adapter = source("validator/src/repository_fit/product_adapter.rs");
    let protocol = source("validator/src/repository_fit/product_adapter/protocol.rs");
    let effects = source("validator/src/repository_fit/local/effects.rs");
    assert!(adapter.contains("pub(crate) use protocol"));
    assert!(!adapter.contains("pub use protocol"));
    assert!(protocol.contains("pub(crate) struct OpaqueFitApplyRequest"));
    assert!(protocol.contains("repository-fit-mode-bound-verification-v1"));
    assert!(protocol.contains("byte_verification_sha256"));
    assert!(!protocol.contains("impl Clone for OpaqueFitApplyRequest"));
    assert!(!protocol.contains("derive(Clone"));
    assert!(protocol.contains("No method on this type\n/// performs an effect"));
    assert!(effects.contains("renameatx_np"));
    assert!(effects.contains("libc::RENAME_SWAP"));
    assert!(effects.contains("libc::RENAME_EXCL"));
    assert!(effects.contains("if !self.mutation_lease"));
    assert!(effects.contains("pub(crate) fn open_for_test"));
    assert!(effects.contains("metadata.permissions().mode() & 0o7777"));
    for prohibited in [
        "RootEffectGrant::issue",
        "with_root_grant",
        "Command::new",
        "git reset",
        "git clean",
        "git checkout",
    ] {
        assert!(!adapter.contains(prohibited));
        assert!(!protocol.contains(prohibited));
        assert!(!effects.contains(prohibited));
    }
}

#[test]
fn canonical_template_bytes_are_compile_time_bound_and_manifest_checked() {
    let catalog = source("validator/src/repository_fit/product_adapter/catalog.rs");
    let build = source("validator/build.rs");
    let classifier = source("validator/build_support/repository_fit_template_sources.rs");
    assert!(catalog.contains("repository_fit_template_catalog.rs"));
    assert!(!catalog.contains("../../../../templates/"));
    assert!(build.contains("repository_fit_template_sources::generate"));
    assert!(classifier.contains("symlink_metadata"));
    assert!(classifier.contains("metadata.nlink() != 1"));
    assert!(classifier.contains("metadata.dev() != root_device"));
    assert!(classifier.contains("no_follow_nonblock_flags"));
    assert!(classifier.contains("first != second"));
    assert!(classifier.contains("observed_paths != manifest_set"));
    assert!(classifier.contains("verify_final_source_tree(templates, &observed)"));
    assert!(classifier.contains("final_tree.directories != expected.directories"));
    assert!(classifier.contains("template source tree changed after inspection"));
    assert!(classifier.contains("clean_authority_outputs(output)?"));
    assert!(classifier.contains("finish_operation(output, result"));
    assert!(classifier.contains("authority-bearing output cleanup failed"));
    assert!(classifier.contains("catch_unwind(AssertUnwindSafe"));
    assert!(catalog.contains("catalog_sources_exact != manifest_exact"));
    assert!(catalog.contains("catalog_sources_folded != manifest_folded"));
    assert!(catalog.contains("TemplateSourceKind::Regular"));
    assert!(!catalog.contains("std::env"));
    assert!(!catalog.contains("std::fs"));
}

#[test]
fn root_owned_public_wiring_remains_unmodified_by_the_candidate() {
    let public_mod = source("validator/src/cli/successor_public/mod.rs");
    let library = source("validator/src/lib.rs");
    assert!(!public_mod.lines().any(|line| line.trim() == "mod fit;"));
    assert!(!public_mod.contains("fit::inspect"));
    assert!(!public_mod.contains("fit::prepare_apply"));
    assert!(library.contains("pub mod repository_fit;"));
}

#[test]
fn public_fit_candidate_is_projection_and_preparation_only() {
    let fit = source("validator/src/cli/successor_public/fit.rs");
    assert!(fit.contains("pub(super) fn inspect"));
    assert!(fit.contains("pub(super) fn plan"));
    assert!(fit.contains("pub(super) fn verify"));
    assert!(fit.contains("pub(super) fn prepare_apply"));
    assert!(!fit.contains("repository_fit::apply("));
    assert!(!fit.contains("LocalEffects"));
    assert!(fit.contains("issues no\n//! root grant or permit"));
}
