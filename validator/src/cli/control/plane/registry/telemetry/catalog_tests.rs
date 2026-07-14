use super::catalog::*;
use crate::cli::control::plane::operation::ControlOperation::*;

#[test]
fn command_parts_cover_every_control_operation() {
    for (operation, command, subcommand) in [
        (LawGraphStrict, "ultragoal law", "graph --strict"),
        (LawCheckGate, "ultragoal law", "check --gate"),
        (LawCheckAll, "ultragoal law", "check --all"),
        (
            StandardsCheckStrict,
            "ultragoal standards",
            "check --strict",
        ),
        (
            SourceObligationsCheckStrict,
            "ultragoal source-obligations",
            "check --strict",
        ),
        (
            FoundationalTraceCheckStrict,
            "ultragoal foundational-trace",
            "check --strict",
        ),
        (
            NamespaceCheckStrict,
            "ultragoal namespace",
            "check --strict",
        ),
        (
            TypedBoundariesCheckStrict,
            "ultragoal typed-boundaries",
            "check --strict",
        ),
        (LineCapsCheckStrict, "ultragoal line-caps", "check --strict"),
        (CoverageProve, "ultragoal coverage", "prove"),
        (ProductInit, "ultragoal product", "init"),
        (ProductProveFitness, "ultragoal product", "prove-fitness"),
        (ProductProveCohesion, "ultragoal product", "prove-cohesion"),
        (ProductProveSuccess, "ultragoal product", "prove-success"),
        (ProductCheckClaims, "ultragoal product", "check-claims"),
        (CapabilityDiscover, "ultragoal capability", "discover"),
        (CapabilityProve, "ultragoal capability", "prove"),
        (InstallAudit, "ultragoal install", "audit"),
        (CacheAudit, "ultragoal cache", "audit"),
        (RegistryProbe, "ultragoal registry", "probe"),
        (AppSurfaceProbe, "ultragoal app-surface", "probe"),
        (TargetRepoAudit, "ultragoal target-repo", "audit"),
        (PackageInventory, "ultragoal package", "inventory"),
        (PackageVerify, "ultragoal package", "verify"),
        (PacketBuild, "ultragoal packet", "build"),
        (PacketVerify, "ultragoal packet", "verify"),
        (ReceiptsVerify, "ultragoal receipts", "verify"),
        (FixturesRed, "ultragoal fixtures", "red"),
        (FixturesGreen, "ultragoal fixtures", "green"),
        (FixturesTamper, "ultragoal fixtures", "tamper"),
        (FixturesAll, "ultragoal fixtures", "all"),
        (CleanRoomRebuild, "ultragoal clean-room", "rebuild"),
        (ClaimCeilingCompute, "ultragoal claim-ceiling", "compute"),
        (Explain, "ultragoal", "explain"),
        (FailureCapture, "ultragoal failure", "capture"),
        (FailurePromote, "ultragoal failure", "promote"),
        (IssueCheckLifecycle, "ultragoal issue", "check-lifecycle"),
        (
            UpdateGoalEligibility,
            "ultragoal update-goal",
            "eligibility",
        ),
        (SelfAuditStrict, "ultragoal self", "audit --strict"),
        (SelfLawGraphStrict, "ultragoal self", "law-graph --strict"),
        (SelfFixturesRed, "ultragoal self fixtures", "red"),
        (SelfFixturesGreen, "ultragoal self fixtures", "green"),
        (SelfFixturesTamper, "ultragoal self fixtures", "tamper"),
        (
            SelfUpdateGoalEligibility,
            "ultragoal self",
            "update-goal eligibility",
        ),
    ] {
        assert_eq!(command_parts(operation), (command, subcommand));
    }
}

#[test]
fn catalog_classifies_surfaces_claims_and_repairs() {
    assert_eq!(surface(AppSurfaceProbe), "codex_desktop_app_surface");
    assert_eq!(surface(RegistryProbe), "codex_desktop_plugin_registry");
    assert_eq!(surface(InstallAudit), "installed_package_surface");
    assert_eq!(surface(CacheAudit), "plugin_cache_surface");
    assert_eq!(
        surface(UpdateGoalEligibility),
        "source_local_update_goal_control_plane"
    );
    assert_eq!(surface(CoverageProve), "source_package_control_plane");

    assert_eq!(
        check_id(AppSurfaceProbe),
        "app-surface-probe-observability-binding"
    );
    assert_eq!(
        check_id(RegistryProbe),
        "registry-probe-observability-binding"
    );
    assert_eq!(
        check_id(UpdateGoalEligibility),
        "update-goal-eligibility-observability-binding"
    );
    assert_eq!(
        check_id(SelfUpdateGoalEligibility),
        "self-update-goal-eligibility-observability-binding"
    );
    assert_eq!(
        check_id(CoverageProve),
        "cli-control-plane-observability-binding"
    );

    assert_eq!(claim_id(RegistryProbe), "app_registry_or_reviewer_exposure");
    assert_eq!(
        claim_id(AppSurfaceProbe),
        "app_registry_or_reviewer_exposure"
    );
    assert_eq!(claim_id(UpdateGoalEligibility), "update_goal_eligibility");
    assert_eq!(
        claim_id(SelfUpdateGoalEligibility),
        "update_goal_eligibility"
    );
    assert_eq!(claim_id(CoverageProve), "source_local_control_plane");

    for (why, class) in [
        ("none", "none"),
        ("coverage receipt stale", "coverage_blocker"),
        ("final_packet missing", "final_packet_blocker"),
        (
            "registry exposure absent",
            "external_live_surface_unavailable",
        ),
        ("digest mismatch", "wrong_digest"),
        (
            "transaction proof absent",
            "transactional_finalization_blocked",
        ),
        ("red_fixture report stale", "red_fixture_report_blocker"),
        ("other", "control_plane_evidence_failed"),
    ] {
        assert_eq!(failure_class(why), class);
    }

    assert_eq!(where_failed(CoverageProve, "pass"), "none");
    assert_eq!(
        where_failed(RegistryProbe, "fail"),
        "registry_probe#/evidence_graph/items/registry_exposure/failures/0"
    );
    assert_eq!(
        where_failed(CoverageProve, "fail"),
        "cli_control_plane#/failure/coverage_prove"
    );
    assert_eq!(next_repair(CoverageProve, "pass"), "none");
    assert!(next_repair(AppSurfaceProbe, "fail").contains("app registry"));
    assert!(next_repair(RegistryProbe, "fail").contains("registry/reviewer"));
    assert!(next_repair(UpdateGoalEligibility, "fail").contains("exact coverage"));
    assert!(next_repair(SelfUpdateGoalEligibility, "fail").contains("red fixture report"));
    assert!(next_repair(CoverageProve, "fail").contains("coverage_prove"));
}

#[test]
fn catalog_enforces_claim_ceiling_and_bounded_runtime_classes() {
    assert!(
        claim_impact(AppSurfaceProbe, "pass")
            .contains("no_readiness_release_completion_update_goal")
    );
    assert!(
        claim_impact(RegistryProbe, "pass").contains("no_readiness_release_completion_update_goal")
    );
    assert!(claim_impact(UpdateGoalEligibility, "pass").contains("no_update_goal_call"));
    assert!(claim_impact(CoverageProve, "pass").contains("supports_source_local_control_plane"));
    assert!(claim_impact(RegistryProbe, "fail").contains("blocks_registry_reviewer"));
    assert!(claim_impact(UpdateGoalEligibility, "fail").contains("blocks_update_goal"));
    assert!(claim_impact(CoverageProve, "fail").contains("blocks_source_local"));

    assert_eq!(
        supported_claims(AppSurfaceProbe, "pass"),
        vec!["app_surface_same_surface_observation".to_string()]
    );
    assert_eq!(
        supported_claims(RegistryProbe, "pass"),
        vec!["live_registry_or_reviewer_exposure_same_surface_pass".to_string()]
    );
    assert_eq!(
        supported_claims(UpdateGoalEligibility, "pass"),
        vec!["update_goal_control_plane_observability_binding".to_string()]
    );
    assert_eq!(
        supported_claims(CoverageProve, "pass"),
        vec!["source_local_control_plane_observability_binding".to_string()]
    );
    assert!(supported_claims(CoverageProve, "fail").is_empty());

    assert_eq!(
        saturation(AppSurfaceProbe),
        "external_live_app_surface_probe_serial_typed"
    );
    assert_eq!(
        saturation(RegistryProbe),
        "external_live_registry_probe_serial_typed_no_capability"
    );
    assert_eq!(
        saturation(InstallAudit),
        "external_live_surface_audit_serial_typed"
    );
    assert_eq!(
        saturation(UpdateGoalEligibility),
        "shared_authority_write_serial_update_goal_control_plane"
    );
    assert_eq!(
        saturation(CoverageProve),
        "shared_authority_write_serial_control_plane"
    );
    assert_eq!(cache_mode(RegistryProbe), "registry_probe_no_cache");
    assert_eq!(cache_mode(CoverageProve), "control_plane_no_cache");
    assert!(registry_surface(AppSurfaceProbe));
    assert!(registry_surface(RegistryProbe));
    assert!(!registry_surface(CoverageProve));
}
