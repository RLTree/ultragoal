use serde_json::Value;
use std::collections::BTreeSet;
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

fn declaration<'a>(source: &'a str, marker: &str) -> (&'a str, &'a str) {
    let start = source
        .find(marker)
        .unwrap_or_else(|| panic!("missing {marker}"));
    let end = start + source[start..].find("\n}").unwrap() + 2;
    let prefix_start = source[..start]
        .rfind("\n\n")
        .map_or(0, |boundary| boundary + 2);
    (&source[prefix_start..start], &source[start..end])
}

#[test]
fn fixture_catalog_is_the_exact_apply_mediation_matrix() {
    let value: Value = serde_json::from_str(&source(
        "fixtures/repository-fit-apply-mediation/cases.json",
    ))
    .unwrap();
    assert_eq!(
        value["schema_version"],
        "RepositoryFitApplyMediationFixtureCatalog-v1"
    );
    assert_eq!(
        value["temporary_root"],
        "/tmp/hul-repository-fit-apply-mediation-058"
    );
    assert_eq!(value["claim_effect"], "none");
    assert_eq!(value["production_permit_issuer"], "absent");
    assert_eq!(value["public_apply_dispatch"], "absent");
    let observed = value["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    let expected = [
        "accepted-plan-digest-substitution",
        "already-fitted-idempotent-apply",
        "apply-failure-complete-rollback",
        "canonical-plan-record-substitution",
        "case-aliased-plan-path",
        "concurrent-identical-contenders",
        "content-free-diagnostics",
        "desired-byte-substitution",
        "desired-mode-substitution",
        "descriptor-case-alias-during-capture",
        "descriptor-final-within-capture-swap",
        "descriptor-leaf-a-b-hybrid-during-capture",
        "descriptor-missing-ancestor-publish-during-capture",
        "descriptor-parent-a-b-hybrid-during-capture",
        "descriptor-postflight-within-capture-swap",
        "descriptor-shared-ancestor-swap-during-capture",
        "dirty-repository-preserved",
        "duplicate-semantic-request-issuance",
        "expired-permit",
        "fresh-repository-exact-apply",
        "git-metadata-mutation",
        "green-effect-without-write",
        "invalid-validity-window",
        "managed-ancestor-case-alias-after-final-effect",
        "managed-ancestor-case-alias-before-effect",
        "managed-ancestor-exact-creation",
        "managed-ancestor-mode-drift-after-final-effect",
        "managed-ancestor-mode-drift-before-effect",
        "managed-ancestor-replacement-after-final-effect",
        "managed-ancestor-replacement-before-effect",
        "managed-ancestor-special-substitution-after-final-effect",
        "managed-ancestor-special-substitution-before-effect",
        "managed-ancestor-symlink-substitution-after-final-effect",
        "managed-ancestor-symlink-substitution-before-effect",
        "managed-shared-ancestor-replacement-after-final-effect",
        "managed-shared-ancestor-replacement-before-effect",
        "matching-bytes-mode-only-apply",
        "missing-exclusive-mutation-lease",
        "missing-permit",
        "mode-failure-complete-rollback",
        "move-only-permit-and-request",
        "mutation-lease-instance-substitution",
        "no-shell-network-or-git-mutation-authority",
        "non-apple-fail-closed",
        "nonce-reuse",
        "not-yet-valid-permit",
        "observed-mode-substitution",
        "out-of-plan-effect-request",
        "partial-retrofit-exact-apply",
        "permit-debug-redaction",
        "plan-mutation-substitution",
        "private-success-construction",
        "production-permit-issuer-absent",
        "protected-fifo-refusal",
        "protected-final-green-a-b-swap",
        "protected-final-green-recheck-late-write",
        "protected-hardlink-refusal",
        "protected-late-same-inode-write-issuance",
        "protected-managed-ancestor-descendant-drift",
        "protected-managed-ancestor-descendant-preserved",
        "protected-mutate-restore-aba",
        "protected-permit-issuance-a-b-swap",
        "protected-postflight-hidden-write-a-b-swap",
        "protected-postflight-late-mode-change",
        "protected-pre-effect-a-b-swap",
        "protected-pre-effect-late-length-write",
        "protected-reconciliation-a-b-swap",
        "protected-reconciliation-late-write",
        "protected-rollback-a-b-swap",
        "protected-rollback-late-write",
        "protected-socket-refusal",
        "protected-symlink-refusal",
        "protected-two-file-hybrid-issuance",
        "protected-unowned-path-mutation",
        "public-apply-dispatch-absent",
        "repository-root-replacement-after-effect",
        "repository-root-replacement-before-effect",
        "rollback-failure-ambiguous",
        "same-content-target-rename-replacement",
        "short-permit-nonce",
        "short-root-secret",
        "structurally-equal-request-substitution",
        "target-cross-device-substitution",
        "target-content-complete-collect-race",
        "target-final-green-post-protected-aba",
        "target-fifo-substitution",
        "target-hardlink-substitution",
        "target-length-complete-collect-race",
        "target-link-complete-collect-race",
        "target-mode-complete-collect-race",
        "target-ownership-complete-collect-race",
        "target-path-complete-collect-race",
        "target-permission-drift",
        "target-rollback-attribution-aba",
        "target-reconciliation-authorized-snapshot-gaps",
        "target-same-inode-mutate-restore-aba",
        "target-socket-substitution",
        "target-special-complete-collect-race",
        "target-symlink-substitution",
        "target-two-leaf-alternating-hybrid",
        "template-catalog-substitution",
        "template-manifest-substitution",
        "template-source-row-substitution",
        "traversal-plan-path",
        "undeclared-write-ambiguous",
        "unowned-overwrite-conflict",
        "verify-as-apply",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected);
    assert_eq!(value["cases"].as_array().unwrap().len(), expected.len());
}

#[test]
fn request_permit_and_lease_are_move_only_internal_authority() {
    let adapter = source("validator/src/repository_fit/product_adapter.rs");
    let protocol = source("validator/src/repository_fit/product_adapter/protocol.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit.rs");
    assert!(adapter.lines().any(|line| line == "mod root_permit;"));
    assert!(!adapter.contains("pub mod root_permit"));
    assert!(!adapter.contains("pub(crate) mod root_permit"));

    let (request_prefix, _) = declaration(&protocol, "pub(crate) struct OpaqueFitApplyRequest");
    let (permit_prefix, permit_declaration) =
        declaration(&permit, "pub(crate) struct RepositoryFitApplyPermit");
    let (lease_prefix, lease_declaration) =
        declaration(&permit, "pub(crate) struct RepositoryFitMutationLease");
    for prefix in [request_prefix, permit_prefix, lease_prefix] {
        assert!(!prefix.contains("#[derive"));
    }
    for type_name in [
        "OpaqueFitApplyRequest",
        "RepositoryFitApplyPermit",
        "RepositoryFitMutationLease",
    ] {
        assert!(!protocol.contains(&format!("impl Clone for {type_name}")));
        assert!(!permit.contains(&format!("impl Clone for {type_name}")));
        assert!(!protocol.contains(&format!("impl Copy for {type_name}")));
        assert!(!permit.contains(&format!("impl Copy for {type_name}")));
        assert!(!protocol.contains(&format!("impl Serialize for {type_name}")));
        assert!(!permit.contains(&format!("impl Serialize for {type_name}")));
        assert!(!protocol.contains(&format!("impl Deserialize for {type_name}")));
        assert!(!permit.contains(&format!("impl Deserialize for {type_name}")));
    }
    assert!(!protocol.contains("use serde::Deserialize"));
    assert!(!permit.contains("use serde::Deserialize"));
    assert!(
        permit_declaration
            .lines()
            .all(|line| !line.contains("pub "))
    );
    assert!(lease_declaration.lines().all(|line| !line.contains("pub ")));
    assert!(permit.contains(".field(\"permit_id\", &\"[bound]\")"));
    assert!(permit.contains(".field(\"nonce\", &\"[redacted]\")"));
    assert!(permit.contains(".field(\"authority\", &\"[redacted]\")"));
}

#[test]
fn permit_binds_every_authority_dimension_and_exact_request_instance() {
    let protocol = source("validator/src/repository_fit/product_adapter/protocol.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit.rs");
    for dimension in [
        "request_id",
        "context_id",
        "candidate_id",
        "repository_root_id",
        "worktree_root_id",
        "root_binding",
        "plan_record_sha256",
        "plan_sha256",
        "accepted_plan_sha256",
        "desired_state_sha256",
        "source_manifest_sha256",
        "source_catalog_sha256",
        "source_authority_sha256",
        "target_prestate_sha256",
        "allowed_mutation_set_sha256",
        "rollback_policy_sha256",
        "protected_prestate_sha256",
    ] {
        assert!(
            permit.contains(&format!("{dimension}:")),
            "missing {dimension}"
        );
    }
    assert!(protocol.contains("static NEXT_APPLY_REQUEST_ISSUANCE: AtomicU64"));
    assert!(protocol.contains("compare_exchange("));
    assert!(protocol.contains("REQUEST_STAGE_PREPARED"));
    assert!(protocol.contains("REQUEST_STAGE_IN_FLIGHT"));
    assert!(permit.contains("Arc::ptr_eq(&request.seal, &permit.seal)"));
    assert!(permit.contains("Arc::ptr_eq(&request.seal, &lease.seal)"));
    assert!(permit.contains("Arc::ptr_eq(&permit.authority, &lease.authority)"));
    assert!(permit.contains("nonce_reservation"));
    assert!(permit.contains("effect_reservation"));
    assert!(permit.contains("reservations.contains(&nonce_reservation)"));
    assert!(permit.contains("reservations.contains(&effect_reservation)"));
}

#[test]
fn apply_revalidates_before_effect_and_has_only_exact_terminal_states() {
    let permit = source("validator/src/repository_fit/product_adapter/root_permit.rs");
    let apply_start = permit.find("pub(crate) fn apply_with_root_permit").unwrap();
    let preflight_start = permit[apply_start..]
        .find("let preflight = preflight(")
        .unwrap();
    let begin_start = permit[apply_start..].find("request.seal.begin()").unwrap();
    assert!(preflight_start < begin_start);
    let apply_end = permit[apply_start..].find("\nfn preflight").unwrap() + apply_start;
    let apply_body = &permit[apply_start..apply_end];
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
    assert!(permit.contains("fn visit_protected_descriptor("));
    assert!(permit.contains("let duplicated = unsafe { libc::dup(directory.as_raw_fd()) }"));
    assert!(permit.contains("libc::fdopendir"));
    assert!(permit.contains("libc::readdir"));
    assert!(permit.contains("fn open_target_at_bytes("));
    assert!(permit.contains("fn named_object_at_bytes("));
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
    assert!(
        permit.contains(
            "let first = collect_target_descriptor_chain_for_paths(root, target_paths)?;"
        )
    );
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
    assert!(
        permit.contains("let authorized_target = match effects.revalidate_authorized_target()")
    );
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
    let controls = source("validator/src/repository_fit/product_adapter/tests/root_permit.rs");
    assert!(
        controls
            .contains("fn reconciliation_binds_every_target_collect_to_the_authorized_snapshot()")
    );
    assert!(controls.contains(
        "mediator-owned rollback ctime is valid because authorized_target records the post-rollback snapshot"
    ));
}

#[test]
fn effect_scope_success_construction_and_diagnostics_fail_closed() {
    let adapter = source("validator/src/repository_fit/product_adapter.rs");
    let fit = source("validator/src/cli/successor_public/fit.rs");
    let public = source("validator/src/cli/successor_public/mod.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit.rs");
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
    assert!(public.contains("if invocation.effect != EffectClass::Read"));
    assert!(!public.contains("SuccessorCommand::Fit(FitAction::Apply) =>"));
    assert!(!fit.contains("apply_with_root_permit"));
}

#[test]
fn production_issuance_and_public_apply_remain_explicitly_unwired() {
    let adapter = source("validator/src/repository_fit/product_adapter.rs");
    let repository_fit = source("validator/src/repository_fit/mod.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit.rs");
    let tests = source("validator/src/repository_fit/product_adapter/tests.rs");
    let direct = source("validator/src/repository_fit/product_adapter/tests/root_permit.rs");
    assert!(permit.contains("#[cfg(test)]\npub(super) struct TestRepositoryFitPermitAuthority"));
    assert!(permit.contains("#[cfg(test)]\npub(super) fn duplicate_authorization_for_test"));
    assert!(!permit.contains("pub(super) struct RepositoryFitPermitAuthority"));
    assert!(!adapter.contains("pub(crate) use root_permit"));
    assert!(!repository_fit.contains("RepositoryFitApplyPermit"));
    assert!(!repository_fit.contains("apply_with_root_permit"));
    assert!(tests.contains("#[path = \"tests/root_permit.rs\"]"));
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
