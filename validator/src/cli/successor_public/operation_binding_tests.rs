use super::*;
use crate::cli::successor::OutputMode;
use crate::cli::successor::command_contract::PackageAction;

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
            SuccessorCommand::Observe(ObserveAction::Query),
            EffectClass::Read,
        )),
        Some(PublicOperation::ObservabilityQuery),
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
        invocation(SuccessorCommand::Fit(FitAction::Apply), EffectClass::Read),
    ] {
        assert_eq!(bind(&invocation), None);
    }
}

#[test]
fn unsupported_api_families_do_not_gain_dispatcher_authority() {
    let active = active_api_identifiers();
    for api in [
        "PackageSnapshot",
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
    for group in ["observe", "package", "eval", "migrate", "prove"] {
        assert!(!active_command_groups().contains(group));
    }
}
