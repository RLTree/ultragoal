use super::*;

pub(crate) fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

pub(crate) fn source(relative: &str) -> String {
    fs::read_to_string(repository_root().join(relative)).unwrap()
}

pub(crate) fn declaration<'a>(source: &'a str, marker: &str) -> (&'a str, &'a str) {
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
pub(crate) fn fixture_catalog_is_the_exact_apply_mediation_matrix() {
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
pub(crate) fn request_permit_and_lease_are_move_only_internal_authority() {
    let adapter = source("validator/src/repository_fit/product_adapter/mod.rs");
    let protocol = source("validator/src/repository_fit/product_adapter/protocol/mod.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit/mod.rs");
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
pub(crate) fn permit_binds_every_authority_dimension_and_exact_request_instance() {
    let protocol = source("validator/src/repository_fit/product_adapter/protocol/mod.rs");
    let permit = source("validator/src/repository_fit/product_adapter/root_permit/mod.rs");
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
