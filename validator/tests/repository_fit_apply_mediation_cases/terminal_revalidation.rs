use super::*;

#[test]
pub(crate) fn apply_revalidates_before_effect_and_has_only_exact_terminal_states() {
    let application =
        source("validator/src/repository_fit/product_adapter/root_permit/permitted_application.rs");
    let preflight =
        source("validator/src/repository_fit/product_adapter/root_permit/effect_preflight.rs");
    let descriptor_walk = source(
        "validator/src/repository_fit/product_adapter/root_permit/protected/descriptor_walk.rs",
    );
    let path_capture = source(
        "validator/src/repository_fit/product_adapter/root_permit/protected/path_capture.rs",
    );
    let identity =
        source("validator/src/repository_fit/product_adapter/root_permit/permit/identity.rs");
    assert!(application.contains("let preflight = preflight("));
    assert!(application.contains("request.seal.begin()"));
    assert!(preflight.contains("revalidate_apply_request(context, request)?"));
    assert!(preflight.contains("ProtectedCaptureBoundary::ImmediatePreEffect"));
    assert!(preflight.contains("capture_target_descriptor_chain"));
    assert!(descriptor_walk.contains("libc::O_NOFOLLOW"));
    assert!(descriptor_walk.contains("stable_descriptor_file_digest"));
    assert!(descriptor_walk.contains("if enumerate_protected_entries(directory)? != entries"));
    assert!(path_capture.contains("fn open_target_at_bytes("));
    assert!(path_capture.contains("fn named_versioned_object_at_bytes("));
    assert!(path_capture.contains("libc::openat"));
    assert!(path_capture.contains("libc::fdopendir"));
    assert!(path_capture.contains("libc::readdir"));
    assert!(path_capture.contains("libc::AT_SYMLINK_NOFOLLOW"));
    assert!(identity
        .contains("let first = collect_target_descriptor_chain_for_paths(root, target_paths)?;"));
    assert!(identity.contains(
        "let final_recheck = collect_target_descriptor_chain_for_paths(root, target_paths)?;"
    ));
    assert!(identity.contains("if first.snapshot != final_recheck.snapshot"));
    assert!(preflight.contains("request.seal.rolled_back()"));
    assert!(preflight.contains("request.seal.ambiguous()"));
    assert!(preflight.contains("AdapterErrorId::ApplyOutcomeAmbiguous"));
}

#[test]
pub(crate) fn public_apply_mediates_diagnostics_without_exposing_private_authority() {
    let adapter = source("validator/src/repository_fit/product_adapter/mod.rs");
    let repository_fit = source("validator/src/repository_fit/mod.rs");
    let public = source("validator/src/cli/successor_public/output_limit.rs");
    let fit_apply = source("validator/src/cli/successor_public/fit/plan_input_limit.rs");
    let public_effect = source("validator/src/cli/successor_public/fit/authority/public_effect.rs");
    let supported_effect =
        source("validator/src/cli/successor_public/fit/authority/supported/state_components.rs");
    assert!(public.contains("SuccessorCommand::Fit(FitAction::Apply) => fit::apply"));
    assert!(fit_apply.contains("authority::recover_pending(context, home)"));
    assert!(fit_apply.contains("authority::execute(context, prepared, home)"));
    assert!(public_effect.contains("supported::execute(context, prepared, home)"));
    assert!(public_effect.contains("supported::recover_pending(context, home)"));
    assert!(supported_effect.contains("prepare_recovery_intent(context, &prepared)"));
    assert!(supported_effect.contains("execute_prepared_apply("));
    assert!(supported_effect.contains("state.persist_pending(&envelope)?"));
    assert!(supported_effect.contains("recover_record(context, &state, pending)"));
    assert!(supported_effect.contains("recover_prepared_apply("));
    for diagnostic in [
        "ApplyPermitMissing",
        "ApplyPermitInvalid",
        "ApplyPermitExpired",
        "ApplyPermitReplayed",
        "ApplyLeaseInvalid",
        "ApplyMutationScopeViolation",
        "ApplyOutcomeInvalid",
        "ApplyRolledBack",
        "ApplyOutcomeAmbiguous",
    ] {
        assert!(
            public_effect.contains(diagnostic),
            "missing diagnostic {diagnostic}"
        );
    }
    assert!(!adapter.contains("pub(crate) use root_permit"));
    assert!(!repository_fit.contains("RepositoryFitApplyPermit"));
    assert!(!repository_fit.contains("apply_with_root_permit"));
}
