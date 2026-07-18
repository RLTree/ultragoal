use super::*;

#[test]
pub(crate) fn apply_revalidates_before_effect_and_has_only_exact_terminal_states() {
    let permit = rust_tree("validator/src/repository_fit/product_adapter/root_permit");
    let application =
        source("validator/src/repository_fit/product_adapter/root_permit/permitted_application.rs");
    let apply_start = application
        .find("pub(crate) fn apply_with_root_permit")
        .unwrap();
    let preflight_start = application[apply_start..]
        .find("let preflight = preflight(")
        .unwrap();
    let begin_start = application[apply_start..]
        .find("request.seal.begin()")
        .unwrap();
    assert!(preflight_start < begin_start);
    let apply_body = &application[apply_start..];
    let begin_in_body = apply_body.find("request.seal.begin()").unwrap();
    assert!(!apply_body[begin_in_body..].contains("PreEffectFailure"));
    assert!(permit.contains("revalidate_apply_request(context, request)?"));
    assert!(permit.contains("struct TargetCapture"));
    assert!(permit.contains("struct TargetDescriptorChain"));
    assert!(permit.contains("capture_target_descriptor_chain_for_paths"));
    assert!(permit.contains("libc::openat"));
    assert!(permit.contains("libc::O_NOFOLLOW"));
    assert!(permit.contains("libc::fdopendir"));
    assert!(permit.contains("libc::readdir"));
    assert!(permit.contains("libc::AT_SYMLINK_NOFOLLOW"));
    assert!(permit.contains("stable_descriptor_file_digest"));
    assert!(permit.contains("stable_readlink_at"));
    assert!(permit.contains("chain.revalidate()?"));
    assert!(permit.contains("self.authorized_target.chain.revalidate()"));
    assert!(permit.contains("let before = self.capture_authorized()?;"));
    assert!(permit.contains("let result = self.inner.compare_exchange"));
    assert!(permit.contains("let after = match capture_target_descriptor_chain_for_paths"));
    assert!(permit.contains("valid_target_effect_transition("));
    assert!(permit.matches("capture_protected(").count() >= 10);
    assert!(permit.contains("pub(crate) fn visit("));
    assert!(permit.contains("let duplicated = unsafe { libc::dup(directory.as_raw_fd()) }"));
    assert!(permit.contains("libc::fdopendir"));
    assert!(permit.contains("libc::readdir"));
    assert!(permit.contains("fn open_target_at_bytes("));
    assert!(permit.contains("fn named_versioned_object_at_bytes("));
    assert!(permit.contains("libc::AT_SYMLINK_NOFOLLOW"));
    assert!(permit.contains("if enumerate_protected_entries(directory)? != entries"));
    assert!(permit.contains("struct ProtectedChangeVersion"));
    assert!(permit.contains("change_version: ProtectedChangeVersion"));
    assert!(permit.contains("ctime_seconds"));
    assert!(permit.contains("ctime_nanoseconds"));
    assert!(permit.contains("metadata.ctime()"));
    assert!(permit.contains("metadata.ctime_nsec()"));
    assert!(permit.contains("fn collect_protected("));
    assert!(permit.contains("if first != final_recheck"));
    assert!(permit.contains("fn collect_target_descriptor_chain_for_paths("));
    assert!(permit
        .contains("let first = collect_target_descriptor_chain_for_paths(root, target_paths)?;"));
    assert!(permit.contains(
        "let final_recheck = collect_target_descriptor_chain_for_paths(root, target_paths)?;"
    ));
    assert!(permit.contains("if first.snapshot != final_recheck.snapshot"));
    assert!(permit.contains("protected_before != protected_after"));
    assert!(permit.contains("visited.insert((held.object.device, held.object.inode))"));
    assert!(permit.contains("ManagedProtectedRole::StrictAncestor"));
    assert!(permit.contains("ManagedProtectedRole::ExactLeaf"));
    assert!(permit.contains("MAX_FENCE_ENTRIES"));
    assert!(permit.contains("MAX_FENCE_TOTAL_BYTES"));
    assert!(!permit.contains("fs::read_dir(directory)"));
    assert!(!permit.contains("fs::symlink_metadata(&path)"));
    assert!(!permit.contains("stable_file_digest(path"));
    assert!(!permit.contains("fn require_desired_modes("));
    assert!(permit.contains("require_desired_target(context.worktree_root()"));
    assert!(permit.contains("validate_managed_ancestors"));
    assert!(permit.contains("managed_ancestor_equivalent"));
    assert!(permit.contains("valid_created_managed_ancestor"));
    assert!(permit.contains("CREATED_MANAGED_ANCESTOR_MODE"));
    assert!(permit.contains("if &final_target != expected_target"));
    assert!(permit.contains("let final_target_recheck ="));
    assert!(permit.contains("final_target_recheck != final_target"));
    assert!(permit.contains("let target_after = capture_target(context.worktree_root(), request)"));
    assert!(permit.contains("let authorized_target = match effects.revalidate_authorized_target()"));
    assert!(permit.contains("if target == authorized_target"));
    assert!(permit.contains("&& target_after == authorized_target"));
    assert!(permit.contains("target_rollback_equivalent(&authorized_target"));
    assert!(permit.contains("test_before_final_green_observation();"));
    assert!(permit.contains("test_before_postflight_observation();"));
    assert!(permit.contains("TargetCapturePhase::AfterNamedBeforeOpen"));
    assert!(permit.contains("TargetCapturePhase::AfterParentHeldBeforeDescend"));
    assert!(permit.contains("TargetCapturePhase::AfterLeafHeldBeforeRead"));
    assert!(permit.contains("TargetCapturePhase::AfterLeafRevalidated"));
    assert!(permit.contains("TargetCapturePhase::AfterMissingBeforeRecheck"));
    assert!(permit.contains("TargetCapturePhase::BeforeFinalChainRecheck"));
    assert!(permit.contains("ProtectedCapturePhase::AfterEnumerationBeforeChildOpen"));
    assert!(permit.contains("ProtectedCapturePhase::AfterDirectoryHeldBeforeDescend"));
    assert!(permit.contains("ProtectedCapturePhase::AfterRegularRowRevalidated"));
    assert!(permit.contains("ProtectedCapturePhase::BeforeFinalRecheck"));
    for boundary in [
        "PermitIssuance",
        "ImmediatePreEffect",
        "Postflight",
        "FinalGreen",
        "FinalGreenRecheck",
        "Rollback",
        "AmbiguityReconciliation",
    ] {
        assert!(
            permit.contains(&format!("ProtectedCaptureBoundary::{boundary}")),
            "missing protected boundary {boundary}"
        );
    }
    assert!(permit.contains("revalidate_post_context(context)?"));
    assert!(permit.contains("rollback(transaction, effects)"));
    assert!(permit.contains("target_rollback_equivalent"));
    assert!(permit.contains("protected_before == permit.protected_prestate"));
    assert!(permit.contains("request.seal.rolled_back()"));
    assert!(permit.contains("request.seal.ambiguous()"));
    assert!(permit.contains("AdapterErrorId::ApplyOutcomeAmbiguous"));
    assert!(permit.contains("self.inode == other.inode || (leaf && self.kind == \"regular\")"));
    let controls = rust_tree("validator/src/repository_fit/product_adapter/tests/root_permit");
    assert!(controls
        .contains("fn reconciliation_binds_every_target_collect_to_the_authorized_snapshot()"));
    assert!(controls.contains(
        "mediator-owned rollback ctime is valid because authorized_target records the post-rollback snapshot"
    ));
}

#[test]
pub(crate) fn effect_scope_success_construction_and_diagnostics_fail_closed() {
    let adapter = source("validator/src/repository_fit/product_adapter/mod.rs");
    let fit = rust_tree("validator/src/cli/successor_public/fit");
    let fit_apply = source("validator/src/cli/successor_public/fit/plan_input_limit.rs");
    let public_effect = source("validator/src/cli/successor_public/fit/authority/public_effect.rs");
    let supported_effect =
        source("validator/src/cli/successor_public/fit/authority/supported/state_components.rs");
    let public = source("validator/src/cli/successor_public/output_limit.rs");
    let permit = rust_tree("validator/src/repository_fit/product_adapter/root_permit");
    assert!(permit.contains("struct ScopedEffects"));
    assert!(permit.contains("let forward = &row.forward_expected == expected"));
    assert!(permit.contains("let rollback = *expected == ExpectedContent::ExactDigest"));
    assert!(permit.contains("AdapterErrorId::ApplyMutationScopeViolation"));
    assert_eq!(permit.matches("success_outcome(").count(), 2);
    assert!(permit.contains("schema_version: \"RepositoryFitApplyOutcome-v1\""));
    assert!(permit.contains("effect: \"workspace_write\""));
    assert!(permit.contains("claim_effect: \"none\""));
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
        assert!(adapter.contains(diagnostic));
        assert!(fit.contains(diagnostic));
    }
    for prohibited in [
        "Command::new",
        "TcpStream",
        "UdpSocket",
        "fs::write",
        "fs::rename",
        "fs::remove_file",
        "git reset",
        "git clean",
        "git checkout",
    ] {
        assert!(!permit.contains(prohibited), "prohibited {prohibited}");
    }
    assert!(public.contains("if invocation.effect != EffectClass::Read && !public_fit_apply"));
    assert!(public.contains("SuccessorCommand::Fit(FitAction::Apply) => fit::apply"));
    assert!(!fit.contains("apply_with_root_permit"));
    let apply_start = fit_apply.find("pub(crate) fn apply(").unwrap();
    let apply_end = fit_apply[apply_start..]
        .find("\npub(crate) struct ApplyArguments")
        .unwrap()
        + apply_start;
    let apply = &fit_apply[apply_start..apply_end];
    assert!(apply.contains("authority::recover_pending(context, home)"));
    assert!(apply.contains("authority::execute(context, prepared, home)"));
    assert!(public_effect.contains("supported::execute(context, prepared, home)"));
    assert!(public_effect.contains("supported::recover_pending(context, home)"));
    assert!(supported_effect.contains("prepare_recovery_intent(context, &prepared)"));
    assert!(supported_effect.contains("execute_prepared_apply("));
    assert!(supported_effect.contains("recover_prepared_apply("));
    assert!(supported_effect.contains("state.persist_pending(&envelope)?"));
}

#[test]
pub(crate) fn public_apply_is_root_mediated_and_adapter_authority_remains_private() {
    let adapter = source("validator/src/repository_fit/product_adapter/mod.rs");
    let repository_fit = source("validator/src/repository_fit/mod.rs");
    let permit = rust_tree("validator/src/repository_fit/product_adapter/root_permit");
    let tests = source("validator/src/repository_fit/product_adapter/tests/mod.rs");
    let direct = rust_tree("validator/src/repository_fit/product_adapter/tests/root_permit");
    assert!(permit.contains("#[cfg(test)]\npub(crate) struct TestRepositoryFitPermitAuthority"));
    assert!(permit.contains("#[cfg(test)]\npub(crate) fn duplicate_authorization_for_test"));
    assert!(!permit.contains("pub(crate) struct RepositoryFitPermitAuthority"));
    assert!(!adapter.contains("pub(crate) use root_permit"));
    assert!(!repository_fit.contains("RepositoryFitApplyPermit"));
    assert!(!repository_fit.contains("apply_with_root_permit"));
    assert!(tests.contains("#[path = \"root_permit/mod.rs\"]"));
    for control in [
        "concurrent_identical_contenders_have_one_atomic_winner",
        "permit_and_mutation_lease_must_share_the_exact_request_and_authority_instance",
        "accepted_plan_desired_state_source_set_modes_and_paths_are_individually_bound",
        "same_content_target_replacement_permissions_git_state_and_root_replacement_are_stale",
        "descriptor_capture_rejects_parent_leaf_and_missing_boundary_hybrids_without_effects",
        "within_postflight_and_final_capture_swaps_are_terminally_ambiguous",
        "protected_descriptor_capture_rejects_permit_issuance_ab_swap",
        "protected_descriptor_capture_rejects_immediate_pre_effect_ab_swap",
        "protected_descriptor_capture_cannot_hide_postflight_undeclared_write",
        "protected_descriptor_capture_rejects_final_green_ab_swap",
        "protected_descriptor_capture_rejects_rollback_ab_swap",
        "protected_descriptor_capture_rejects_reconciliation_ab_swap",
        "protected_late_same_inode_write_is_detected_and_left_exactly_observable",
        "protected_two_file_hybrid_collect_is_rejected_without_path_sampling",
        "protected_change_version_rejects_same_inode_mutate_restore_aba",
        "protected_pre_effect_late_length_change_refuses_without_effect",
        "protected_postflight_late_mode_change_cannot_produce_success_or_rollback",
        "protected_final_green_target_overlap_is_caught_by_the_after_collect",
        "protected_rollback_late_write_withholds_complete_rollback",
        "protected_reconciliation_late_write_cannot_fabricate_prior_state",
        "target_complete_double_collect_rejects_two_leaf_alternating_hybrid",
        "target_change_version_rejects_same_inode_mutate_restore_aba",
        "target_complete_collect_rejects_content_length_mode_ownership_link_special_and_path_races",
        "final_green_rechecks_target_after_protected_after_and_rejects_late_target_aba",
        "rollback_withholds_attribution_after_unmediated_same_inode_target_aba",
        "protected_descendants_inside_managed_ancestors_are_preserved_and_bound",
        "protected_links_and_special_objects_fail_closed_before_effects",
        "missing_managed_ancestors_are_created_with_exact_root_ownership_and_mode",
        "every_existing_managed_ancestor_is_bound_before_effect",
        "mutation_after_postflight_cannot_cross_the_final_green_ancestor_boundary",
        "complete_rollback_is_terminal_and_ambiguous_rollback_never_false_passes",
        "green_receipt_without_effect_and_undeclared_write_cannot_fabricate_success",
        "root_replacement_after_the_first_effect_is_terminally_ambiguous",
        "verify_cannot_consume_or_substitute_for_apply_and_secrets_never_echo",
    ] {
        assert!(direct.contains(control), "missing direct control {control}");
    }
    assert!(permit.contains("#[cfg(not(unix))]"));
    assert!(permit.contains("AdapterErrorId::UnsupportedHost"));
}
