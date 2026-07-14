use crate::cli::control::plane::operation::{ControlOperation, REQUIRED_COMMANDS};

#[test]
fn every_control_operation_has_stable_id() {
    let rows = [
        (ControlOperation::LawGraphStrict, "law_graph_strict"),
        (ControlOperation::LawCheckGate, "law_check_gate"),
        (ControlOperation::LawCheckAll, "law_check_all"),
        (
            ControlOperation::StandardsCheckStrict,
            "standards_check_strict",
        ),
        (
            ControlOperation::SourceObligationsCheckStrict,
            "source_obligations_check_strict",
        ),
        (
            ControlOperation::FoundationalTraceCheckStrict,
            "foundational_trace_check_strict",
        ),
        (
            ControlOperation::NamespaceCheckStrict,
            "namespace_check_strict",
        ),
        (
            ControlOperation::TypedBoundariesCheckStrict,
            "typed_boundaries_check_strict",
        ),
        (
            ControlOperation::LineCapsCheckStrict,
            "line_caps_check_strict",
        ),
        (ControlOperation::CoverageProve, "coverage_prove"),
        (ControlOperation::ProductInit, "product_init"),
        (
            ControlOperation::ProductProveFitness,
            "product_prove_fitness",
        ),
        (
            ControlOperation::ProductProveCohesion,
            "product_prove_cohesion",
        ),
        (
            ControlOperation::ProductProveSuccess,
            "product_prove_success",
        ),
        (ControlOperation::ProductCheckClaims, "product_check_claims"),
        (ControlOperation::CapabilityDiscover, "capability_discover"),
        (ControlOperation::CapabilityProve, "capability_prove"),
        (ControlOperation::InstallAudit, "install_audit"),
        (ControlOperation::CacheAudit, "cache_audit"),
        (ControlOperation::RegistryProbe, "registry_probe"),
        (ControlOperation::AppSurfaceProbe, "app_surface_probe"),
        (ControlOperation::TargetRepoAudit, "target_repo_audit"),
        (ControlOperation::PackageInventory, "package_inventory"),
        (ControlOperation::PackageVerify, "package_verify"),
        (ControlOperation::PacketBuild, "packet_build"),
        (ControlOperation::PacketVerify, "packet_verify"),
        (ControlOperation::ReceiptsVerify, "receipts_verify"),
        (ControlOperation::FixturesRed, "fixtures_red"),
        (ControlOperation::FixturesGreen, "fixtures_green"),
        (ControlOperation::FixturesTamper, "fixtures_tamper"),
        (ControlOperation::FixturesAll, "fixtures_all"),
        (ControlOperation::CleanRoomRebuild, "clean_room_rebuild"),
        (
            ControlOperation::ClaimCeilingCompute,
            "claim_ceiling_compute",
        ),
        (ControlOperation::Explain, "explain"),
        (ControlOperation::FailureCapture, "failure_capture"),
        (ControlOperation::FailurePromote, "failure_promote"),
        (
            ControlOperation::IssueCheckLifecycle,
            "issue_check_lifecycle",
        ),
        (
            ControlOperation::UpdateGoalEligibility,
            "update_goal_eligibility",
        ),
        (ControlOperation::SelfAuditStrict, "self_audit_strict"),
        (
            ControlOperation::SelfLawGraphStrict,
            "self_law_graph_strict",
        ),
        (ControlOperation::SelfFixturesRed, "self_fixtures_red"),
        (ControlOperation::SelfFixturesGreen, "self_fixtures_green"),
        (ControlOperation::SelfFixturesTamper, "self_fixtures_tamper"),
        (
            ControlOperation::SelfUpdateGoalEligibility,
            "self_update_goal_eligibility",
        ),
    ];
    for (operation, id) in rows {
        assert_eq!(operation.id(), id);
    }
    assert!(REQUIRED_COMMANDS.contains(&"ultragoal update-goal eligibility"));
    assert!(REQUIRED_COMMANDS.contains(&"ultragoal self update-goal eligibility"));
}
