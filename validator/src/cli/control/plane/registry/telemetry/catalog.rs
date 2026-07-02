use crate::cli::control::plane::types::ControlOperation;

pub(crate) fn command_parts(operation: ControlOperation) -> (&'static str, &'static str) {
    match operation {
        ControlOperation::LawGraphStrict => ("ultragoal law", "graph --strict"),
        ControlOperation::LawCheckGate => ("ultragoal law", "check --gate"),
        ControlOperation::LawCheckAll => ("ultragoal law", "check --all"),
        ControlOperation::StandardsCheckStrict => ("ultragoal standards", "check --strict"),
        ControlOperation::SourceObligationsCheckStrict => {
            ("ultragoal source-obligations", "check --strict")
        }
        ControlOperation::FoundationalTraceCheckStrict => {
            ("ultragoal foundational-trace", "check --strict")
        }
        ControlOperation::NamespaceCheckStrict => ("ultragoal namespace", "check --strict"),
        ControlOperation::TypedBoundariesCheckStrict => {
            ("ultragoal typed-boundaries", "check --strict")
        }
        ControlOperation::LineCapsCheckStrict => ("ultragoal line-caps", "check --strict"),
        ControlOperation::CoverageProve => ("ultragoal coverage", "prove"),
        ControlOperation::ProductInit => ("ultragoal product", "init"),
        ControlOperation::ProductProveFitness => ("ultragoal product", "prove-fitness"),
        ControlOperation::ProductProveCohesion => ("ultragoal product", "prove-cohesion"),
        ControlOperation::ProductProveSuccess => ("ultragoal product", "prove-success"),
        ControlOperation::ProductCheckClaims => ("ultragoal product", "check-claims"),
        ControlOperation::CapabilityDiscover => ("ultragoal capability", "discover"),
        ControlOperation::CapabilityProve => ("ultragoal capability", "prove"),
        ControlOperation::InstallAudit => ("ultragoal install", "audit"),
        ControlOperation::CacheAudit => ("ultragoal cache", "audit"),
        ControlOperation::RegistryProbe => ("ultragoal registry", "probe"),
        ControlOperation::AppSurfaceProbe => ("ultragoal app-surface", "probe"),
        ControlOperation::TargetRepoAudit => ("ultragoal target-repo", "audit"),
        ControlOperation::PackageInventory => ("ultragoal package", "inventory"),
        ControlOperation::PackageVerify => ("ultragoal package", "verify"),
        ControlOperation::PacketBuild => ("ultragoal packet", "build"),
        ControlOperation::PacketVerify => ("ultragoal packet", "verify"),
        ControlOperation::ReceiptsVerify => ("ultragoal receipts", "verify"),
        ControlOperation::FixturesRed => ("ultragoal fixtures", "red"),
        ControlOperation::FixturesGreen => ("ultragoal fixtures", "green"),
        ControlOperation::FixturesTamper => ("ultragoal fixtures", "tamper"),
        ControlOperation::FixturesAll => ("ultragoal fixtures", "all"),
        ControlOperation::CleanRoomRebuild => ("ultragoal clean-room", "rebuild"),
        ControlOperation::ClaimCeilingCompute => ("ultragoal claim-ceiling", "compute"),
        ControlOperation::Explain => ("ultragoal", "explain"),
        ControlOperation::FailureCapture => ("ultragoal failure", "capture"),
        ControlOperation::FailurePromote => ("ultragoal failure", "promote"),
        ControlOperation::IssueCheckLifecycle => ("ultragoal issue", "check-lifecycle"),
        ControlOperation::UpdateGoalEligibility => ("ultragoal update-goal", "eligibility"),
        ControlOperation::SelfAuditStrict => ("ultragoal self", "audit --strict"),
        ControlOperation::SelfLawGraphStrict => ("ultragoal self", "law-graph --strict"),
        ControlOperation::SelfFixturesRed => ("ultragoal self fixtures", "red"),
        ControlOperation::SelfFixturesGreen => ("ultragoal self fixtures", "green"),
        ControlOperation::SelfFixturesTamper => ("ultragoal self fixtures", "tamper"),
        ControlOperation::SelfUpdateGoalEligibility => {
            ("ultragoal self", "update-goal eligibility")
        }
    }
}

pub(crate) fn surface(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "codex_desktop_app_surface",
        ControlOperation::RegistryProbe => "codex_desktop_plugin_registry",
        ControlOperation::InstallAudit => "installed_package_surface",
        ControlOperation::CacheAudit => "plugin_cache_surface",
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility => {
            "source_local_update_goal_control_plane"
        }
        _ => "source_package_control_plane",
    }
}

pub(crate) fn check_id(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "app-surface-probe-observability-binding",
        ControlOperation::RegistryProbe => "registry-probe-observability-binding",
        ControlOperation::UpdateGoalEligibility => "update-goal-eligibility-observability-binding",
        ControlOperation::SelfUpdateGoalEligibility => {
            "self-update-goal-eligibility-observability-binding"
        }
        _ => "cli-control-plane-observability-binding",
    }
}

pub(crate) fn claim_id(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe => {
            "app_registry_or_reviewer_exposure"
        }
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility => {
            "update_goal_eligibility"
        }
        _ => "source_local_control_plane",
    }
}

pub(crate) fn failure_class(why: &str) -> &'static str {
    if why == "none" {
        "none"
    } else if why.contains("coverage") {
        "coverage_blocker"
    } else if why.contains("final_packet") {
        "final_packet_blocker"
    } else if why.contains("registry") || why.contains("exposure") {
        "external_live_surface_unavailable"
    } else if why.contains("digest") {
        "wrong_digest"
    } else if why.contains("transaction") {
        "transactional_finalization_blocked"
    } else if why.contains("red_fixture") {
        "red_fixture_report_blocker"
    } else {
        "control_plane_evidence_failed"
    }
}

pub(crate) fn where_failed(operation: ControlOperation, status: &str) -> String {
    if status == "pass" {
        return "none".to_string();
    }
    if registry_surface(operation) {
        "registry_probe#/evidence_graph/items/registry_exposure/failures/0".to_string()
    } else {
        format!("cli_control_plane#/failure/{}", operation.id())
    }
}

pub(crate) fn next_repair(operation: ControlOperation, status: &str) -> String {
    if status == "pass" {
        return "none".to_string();
    }
    match operation {
        ControlOperation::AppSurfaceProbe => {
            "provide live same-surface app registry proof or keep app-surface and reviewer claims blocked"
                .to_string()
        }
        ControlOperation::RegistryProbe => {
            "provide live same-surface registry/reviewer proof or keep registry/reviewer claims blocked"
                .to_string()
        }
        ControlOperation::UpdateGoalEligibility => "repair same-candidate final packet, exact coverage, registry/app, performance, and transactional finalization blockers before rerunning update-goal eligibility".to_string(),
        ControlOperation::SelfUpdateGoalEligibility => "repair same-candidate source audit, red fixture report, exact coverage, performance, final packet, registry/app, and transactional finalization blockers before rerunning self update-goal eligibility".to_string(),
        _ => format!(
            "repair same-candidate {} evidence graph blockers before rerunning the narrow command",
            operation.id()
        ),
    }
}

pub(crate) fn claim_impact(operation: ControlOperation, status: &str) -> String {
    if status == "pass" {
        return match operation {
            ControlOperation::AppSurfaceProbe => "supports_app_surface_observation_only_no_readiness_release_completion_update_goal",
            ControlOperation::RegistryProbe => "supports_registry_probe_observation_only_no_readiness_release_completion_update_goal",
            ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility => "supports_update_goal_control_plane_observability_only_no_update_goal_call",
            _ => "supports_source_local_control_plane_observability_only_no_readiness_release_completion_update_goal",
        }
        .to_string();
    }
    match operation {
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe => {
            "blocks_registry_reviewer_final_packet_readiness_release_completion_update_goal"
        }
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility => {
            "blocks_update_goal_readiness_release_completion"
        }
        _ => "blocks_source_local_control_plane_readiness_release_completion_update_goal",
    }
    .to_string()
}

pub(crate) fn supported_claims(operation: ControlOperation, status: &str) -> Vec<String> {
    if status != "pass" {
        return Vec::new();
    }
    vec![
        match operation {
            ControlOperation::AppSurfaceProbe => "app_surface_same_surface_observation",
            ControlOperation::RegistryProbe => {
                "live_registry_or_reviewer_exposure_same_surface_pass"
            }
            ControlOperation::UpdateGoalEligibility
            | ControlOperation::SelfUpdateGoalEligibility => {
                "update_goal_control_plane_observability_binding"
            }
            _ => "source_local_control_plane_observability_binding",
        }
        .to_string(),
    ]
}

pub(crate) fn saturation(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::AppSurfaceProbe => "external_live_app_surface_probe_serial_typed",
        ControlOperation::RegistryProbe => {
            "external_live_registry_probe_serial_typed_no_capability"
        }
        ControlOperation::InstallAudit | ControlOperation::CacheAudit => {
            "external_live_surface_audit_serial_typed"
        }
        ControlOperation::UpdateGoalEligibility | ControlOperation::SelfUpdateGoalEligibility => {
            "shared_authority_write_serial_update_goal_control_plane"
        }
        _ => "shared_authority_write_serial_control_plane",
    }
}

pub(crate) fn cache_mode(operation: ControlOperation) -> &'static str {
    match operation {
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe => {
            "registry_probe_no_cache"
        }
        _ => "control_plane_no_cache",
    }
}

pub(crate) fn registry_surface(operation: ControlOperation) -> bool {
    matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    )
}
