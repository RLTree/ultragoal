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
    let apply = function_body(&application, "pub(crate) fn apply_with_root_permit");
    let immediate_pre_effect = function_body(&preflight, "pub(crate) fn preflight");
    assert!(
        apply.find("let preflight = preflight(").unwrap()
            < apply.find("request.seal.begin()").unwrap()
    );
    assert!(!apply[apply.find("request.seal.begin()").unwrap()..].contains("PreEffectFailure"));
    assert!(immediate_pre_effect.contains("revalidate_apply_request(context, request)?"));
    assert!(immediate_pre_effect.contains("ProtectedCaptureBoundary::ImmediatePreEffect"));
    assert!(immediate_pre_effect.contains(".target_chain\n        .revalidate()"));
    assert!(immediate_pre_effect.contains("capture_target_descriptor_chain"));
    assert!(immediate_pre_effect.contains("context\n        .revalidate()"));
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
    let apply = function_body(&fit_apply, "pub(crate) fn apply");
    let authority_execute = function_body(&public_effect, "pub(crate) fn execute");
    let authority_recovery = function_body(&public_effect, "pub(crate) fn recover_pending");
    let supported_execute = function_body(&supported_effect, "pub(crate) fn execute");
    let supported_recovery = function_body(&supported_effect, "pub(crate) fn recover_pending");
    let recovery_record = function_body(&supported_effect, "pub(crate) fn recover_record");
    let diagnostics = function_body(&public_effect, "pub(crate) fn production_outcome");
    assert!(public.contains("if invocation.effect != EffectClass::Read && !public_fit_apply"));
    assert!(public.contains("SuccessorCommand::Fit(FitAction::Apply) => fit::apply"));
    assert!(apply.contains("authority::recover_pending(context, home)"));
    assert!(apply.contains("authority::execute(context, prepared, home)"));
    assert!(authority_execute.contains("supported::execute(context, prepared, home)"));
    assert!(authority_recovery.contains("supported::recover_pending(context, home)"));
    assert!(supported_execute.contains("prepare_recovery_intent(context, &prepared)"));
    assert!(supported_execute.contains("execute_prepared_apply("));
    assert!(supported_execute.contains("state.persist_pending(&envelope)?"));
    assert!(supported_recovery.contains("recover_record(context, &state, pending)"));
    assert!(recovery_record.contains("recover_prepared_apply("));
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
            diagnostics.contains(diagnostic),
            "missing diagnostic {diagnostic}"
        );
    }
    assert!(!adapter.contains("pub(crate) use root_permit"));
    assert!(!repository_fit.contains("RepositoryFitApplyPermit"));
    assert!(!repository_fit.contains("apply_with_root_permit"));
}
