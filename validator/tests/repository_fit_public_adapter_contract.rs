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

fn rust_tree(relative: &str) -> String {
    fn collect(path: &std::path::Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    let mut files = Vec::new();
    collect(&repository_root().join(relative), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
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
    let adapter = source("validator/src/repository_fit/product_adapter/mod.rs");
    let protocol = rust_tree("validator/src/repository_fit/product_adapter/protocol");
    let effects = rust_tree("validator/src/repository_fit/local/effects");
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
    let classifier = format!(
        "{}\n{}",
        source("validator/build_support/repository_fit_template_sources.rs"),
        rust_tree("validator/build_support/repository_fit_templates")
    );
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
fn root_owned_public_wiring_activates_read_routes_and_one_effectful_fit_route() {
    let public_mod = format!(
        "{}\n{}",
        source("validator/src/cli/successor_public/mod.rs"),
        source("validator/src/cli/successor_public/output_limit.rs")
    );
    let library = source("validator/src/lib.rs");
    assert!(public_mod.lines().any(|line| line.trim() == "mod fit;"));
    assert!(public_mod.contains("fit::inspect"));
    assert!(public_mod.contains("fit::plan"));
    assert!(public_mod.contains("fit::verify"));
    assert!(public_mod.contains("fit::apply"));
    assert!(public_mod.contains("with_root_workspace_grant"));
    assert!(public_mod.contains("public_fit_apply"));
    assert!(library.contains("pub mod repository_fit;"));
}

#[test]
fn public_fit_apply_uses_the_sealed_kernel_and_durable_host_recovery() {
    let fit = source("validator/src/cli/successor_public/fit/plan_input_limit.rs");
    let authority = rust_tree("validator/src/cli/successor_public/fit/authority");
    assert!(fit.contains("pub(crate) fn inspect"));
    assert!(fit.contains("pub(crate) fn plan"));
    assert!(fit.contains("pub(crate) fn verify"));
    assert!(fit.contains("pub(crate) fn prepare_apply_with_arguments"));
    assert!(!fit.contains("pub(crate) fn prepare_apply("));
    assert!(fit.contains("pub(crate) fn apply"));
    assert!(fit.contains("bytes.ends_with(b\"\\n\")"));
    assert!(!fit.contains("repository_fit::apply("));
    assert!(!fit.contains("LocalEffects"));
    assert!(authority.contains("execute_prepared_apply"));
    assert!(authority.contains("recover_prepared_apply"));
    assert!(authority.contains("clock_gettime(libc::CLOCK_MONOTONIC"));
    assert!(authority.contains("libc::O_NOFOLLOW"));
    assert!(authority.contains("libc::RENAME_EXCL"));
    assert!(authority.contains("recovery_required"));
    assert!(!authority.contains("create_dir"));
    assert!(!authority.contains("Command::new"));
}
