//! Public operation bindings admit only exact supported command and effect pairs.

use super::*;
use crate::cli::successor::OutputMode;
use crate::cli::successor::command_contract::{EvalAction, MigrateAction, PackageAction};

fn invocation(command: SuccessorCommand, effect: EffectClass) -> ParsedInvocation {
    ParsedInvocation {
        command,
        effect,
        output_mode: OutputMode::Json,
        arguments: Vec::new(),
    }
}

#[test]
fn only_exact_supported_command_effect_pairs_bind() {
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Inspect(InspectTarget::Inception),
            EffectClass::Read,
        )),
        Some(PublicOperation::InceptionInspection),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Observe(ObserveAction::Query),
            EffectClass::Read,
        )),
        Some(PublicOperation::ObservabilityQuery),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Eval(EvalAction::Audit),
            EffectClass::Read,
        )),
        Some(PublicOperation::EvaluationAudit),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Eval(EvalAction::Run),
            EffectClass::WorkspaceWrite,
        )),
        Some(PublicOperation::EvaluationRun),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Migrate(MigrateAction::Plan),
            EffectClass::Read,
        )),
        Some(PublicOperation::MigrationPlan),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Package(PackageAction::Inventory),
            EffectClass::WorkspaceWrite,
        )),
        Some(PublicOperation::PackageInventory),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Package(PackageAction::Build),
            EffectClass::WorkspaceWrite,
        )),
        Some(PublicOperation::PackageBuild),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Package(PackageAction::Verify),
            EffectClass::Read,
        )),
        Some(PublicOperation::PackageVerify),
    );
    assert_eq!(
        bind(&invocation(
            SuccessorCommand::Package(PackageAction::InstallTest),
            EffectClass::WorkspaceWrite,
        )),
        Some(PublicOperation::PackageInstallTest),
    );
    for invocation in [
        invocation(
            SuccessorCommand::Observe(ObserveAction::Export),
            EffectClass::Read,
        ),
        invocation(
            SuccessorCommand::Package(PackageAction::Inventory),
            EffectClass::Read,
        ),
        invocation(
            SuccessorCommand::Package(PackageAction::Build),
            EffectClass::Read,
        ),
        invocation(SuccessorCommand::Fit(FitAction::Apply), EffectClass::Read),
        invocation(
            SuccessorCommand::Migrate(MigrateAction::Apply),
            EffectClass::WorkspaceWrite,
        ),
        invocation(
            SuccessorCommand::Migrate(MigrateAction::Verify),
            EffectClass::Read,
        ),
        invocation(
            SuccessorCommand::Migrate(MigrateAction::Retire),
            EffectClass::Destructive,
        ),
    ] {
        assert_eq!(bind(&invocation), None);
    }
}

#[test]
fn unsupported_api_families_do_not_gain_dispatcher_authority() {
    let active = active_api_identifiers();
    for api in [
        "InstallSnapshot",
        "MarketplaceSnapshot",
        "DiscoveryObservation",
        "RuntimeObservation",
        "FixtureSpec",
        "FixtureScheduler",
        "IsolationLease",
        "ExpectedOutcome",
        "CommandSpec",
        "CapturedRun",
        "ArtifactRef",
        "ArtifactResolver",
        "ExportAdapter",
    ] {
        assert!(
            !active.contains(api),
            "{api} has no supported public operation"
        );
    }
    for group in ["observe", "package", "migrate", "prove"] {
        assert!(!active_command_groups().contains(group));
    }
}
