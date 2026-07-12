use super::model::{
    CheckProfile, CommandDescriptor, EvalAction, FitAction, Group, InspectTarget, MigrateAction,
    ObserveAction, OptionName, OptionSpec, PackageAction, SuccessorCommand, ValueKind,
};
use crate::context::EffectClass;

const TARGET: OptionSpec = option(OptionName::Target, ValueKind::RelativePath, false);
const PLAN: OptionSpec = option(OptionName::Plan, ValueKind::RelativePath, true);
const ACCEPT_PLAN: OptionSpec = option(OptionName::AcceptPlan, ValueKind::Identifier, true);
const CLAIM: OptionSpec = option(OptionName::Claim, ValueKind::Identifier, true);
const FINDING: OptionSpec = option(OptionName::Finding, ValueKind::Identifier, false);
const FILTER: OptionSpec = option(OptionName::Filter, ValueKind::Identifier, false);
const OUTPUT: OptionSpec = option(OptionName::Output, ValueKind::RelativePath, true);
const APPROVE_EXPORT: OptionSpec = option(OptionName::ApproveExport, ValueKind::Flag, true);
const PROVIDER: OptionSpec = option(OptionName::Provider, ValueKind::Identifier, true);
const APPROVE_PUBLISH: OptionSpec = option(OptionName::ApprovePublish, ValueKind::Flag, true);
const SPEC: OptionSpec = option(OptionName::Spec, ValueKind::RelativePath, true);
const INPUT: OptionSpec = option(OptionName::Input, ValueKind::RelativePath, true);
const CANDIDATE: OptionSpec = option(OptionName::Candidate, ValueKind::Identifier, true);
const REGISTRY: OptionSpec = option(OptionName::Registry, ValueKind::RelativePath, false);
const APPROVE_RETIREMENT: OptionSpec = option(OptionName::ApproveRetirement, ValueKind::Flag, true);

const TARGET_OPTION: &[OptionSpec] = &[TARGET];
const FIT_APPLY: &[OptionSpec] = &[TARGET, PLAN, ACCEPT_PLAN];
const STRICT_OPTIONS: &[OptionSpec] = &[TARGET, CLAIM];
const FINDING_OPTION: &[OptionSpec] = &[FINDING];
const CLAIM_OUTPUT: &[OptionSpec] = &[CLAIM, OUTPUT];
const FILTER_OPTION: &[OptionSpec] = &[FILTER];
const EXPORT_OPTIONS: &[OptionSpec] = &[OUTPUT, APPROVE_EXPORT];
const OUTPUT_OPTION: &[OptionSpec] = &[OUTPUT];
const INPUT_OPTION: &[OptionSpec] = &[INPUT];
const PUBLISH_OPTIONS: &[OptionSpec] = &[INPUT, PROVIDER, APPROVE_PUBLISH];
const SPEC_OPTION: &[OptionSpec] = &[SPEC];
const SPEC_OUTPUT: &[OptionSpec] = &[SPEC, OUTPUT];
const INPUT_OUTPUT: &[OptionSpec] = &[INPUT, OUTPUT];
const CANDIDATE_OUTPUT: &[OptionSpec] = &[CANDIDATE, OUTPUT];
const ADAPTER_OPTIONS: &[OptionSpec] = &[SPEC, PROVIDER];
const REGISTRY_OPTION: &[OptionSpec] = &[REGISTRY];
const MIGRATE_APPLY: &[OptionSpec] = &[PLAN, ACCEPT_PLAN];
const MIGRATE_RETIRE: &[OptionSpec] = &[PLAN, APPROVE_RETIREMENT];

const fn option(name: OptionName, kind: ValueKind, required: bool) -> OptionSpec {
    OptionSpec {
        name,
        kind,
        required,
    }
}

macro_rules! command {
    ($command:expr, $sub:expr, $effect:ident, $purpose:literal, $options:expr) => {
        CommandDescriptor {
            command: $command,
            subcommand: $sub,
            effect: EffectClass::$effect,
            purpose: $purpose,
            options: $options,
        }
    };
}

static CATALOG: &[CommandDescriptor] = &[
    command!(
        SuccessorCommand::Inspect(InspectTarget::Summary),
        None,
        Read,
        "Summarize live context, findings, capabilities, and claim ceilings.",
        &[]
    ),
    command!(
        SuccessorCommand::Inspect(InspectTarget::Context),
        Some("context"),
        Read,
        "Inspect candidate-bound live context.",
        &[]
    ),
    command!(
        SuccessorCommand::Inspect(InspectTarget::Inventory),
        Some("inventory"),
        Read,
        "Inspect semantic components and authority conflicts.",
        &[]
    ),
    command!(
        SuccessorCommand::Inspect(InspectTarget::Capabilities),
        Some("capabilities"),
        Read,
        "Inspect exposed tools and supported capability surfaces.",
        &[]
    ),
    command!(
        SuccessorCommand::Inspect(InspectTarget::Findings),
        Some("findings"),
        Read,
        "Inspect typed current findings.",
        &[]
    ),
    command!(
        SuccessorCommand::Inspect(InspectTarget::Claims),
        Some("claims"),
        Read,
        "Inspect per-claim ceilings without promotion.",
        &[]
    ),
    command!(
        SuccessorCommand::Next,
        None,
        Read,
        "Select one deterministic legal action or authority request.",
        &[]
    ),
    command!(
        SuccessorCommand::Fit(FitAction::Inspect),
        Some("inspect"),
        Read,
        "Classify a fresh, partial, compatible, conflicting, or fitted target.",
        TARGET_OPTION
    ),
    command!(
        SuccessorCommand::Fit(FitAction::Plan),
        Some("plan"),
        Read,
        "Compute desired state, exact mutations, conflicts, rollback, and authority needs.",
        TARGET_OPTION
    ),
    command!(
        SuccessorCommand::Fit(FitAction::Apply),
        Some("apply"),
        WorkspaceWrite,
        "Apply only an explicitly accepted, context-bound fit plan.",
        FIT_APPLY
    ),
    command!(
        SuccessorCommand::Fit(FitAction::Verify),
        Some("verify"),
        Read,
        "Verify fitted discovery and behavior without mutation.",
        TARGET_OPTION
    ),
    command!(
        SuccessorCommand::Check(CheckProfile::Routine),
        Some("routine"),
        WorkspaceWrite,
        "Run conservative affected validation with declared local build artifacts.",
        TARGET_OPTION
    ),
    command!(
        SuccessorCommand::Check(CheckProfile::Strict),
        Some("strict"),
        WorkspaceWrite,
        "Run dependency-closed validation for one named claim.",
        STRICT_OPTIONS
    ),
    command!(
        SuccessorCommand::Diagnose,
        None,
        Read,
        "Explain cause, smallest repair, exact rerun, effect, and ceiling.",
        FINDING_OPTION
    ),
    command!(
        SuccessorCommand::Prove,
        None,
        WorkspaceWrite,
        "Execute one claim-specific proof and independent reconciliation packet.",
        CLAIM_OUTPUT
    ),
    command!(
        SuccessorCommand::Observe(ObserveAction::Query),
        Some("query"),
        Read,
        "Query local semantic events without access-metadata writes.",
        FILTER_OPTION
    ),
    command!(
        SuccessorCommand::Observe(ObserveAction::Export),
        Some("export"),
        ExternalWrite,
        "Export explicitly approved semantic data to an external destination.",
        EXPORT_OPTIONS
    ),
    command!(
        SuccessorCommand::Package(PackageAction::Inventory),
        Some("inventory"),
        WorkspaceWrite,
        "Write a deterministic package inventory from canonical source.",
        OUTPUT_OPTION
    ),
    command!(
        SuccessorCommand::Package(PackageAction::Build),
        Some("build"),
        WorkspaceWrite,
        "Build deterministic package bytes and local provenance inputs.",
        OUTPUT_OPTION
    ),
    command!(
        SuccessorCommand::Package(PackageAction::Verify),
        Some("verify"),
        Read,
        "Verify package bytes and inventory without mutation.",
        INPUT_OPTION
    ),
    command!(
        SuccessorCommand::Package(PackageAction::InstallTest),
        Some("install-test"),
        WorkspaceWrite,
        "Exercise an isolated install and write a local verification artifact.",
        INPUT_OUTPUT
    ),
    command!(
        SuccessorCommand::Package(PackageAction::Publish),
        Some("publish"),
        ExternalWrite,
        "Publish only an explicitly approved package through a named provider.",
        PUBLISH_OPTIONS
    ),
    command!(
        SuccessorCommand::Eval(EvalAction::Audit),
        Some("audit"),
        Read,
        "Audit an evaluation specification and dataset without mutation.",
        SPEC_OPTION
    ),
    command!(
        SuccessorCommand::Eval(EvalAction::Run),
        Some("run"),
        WorkspaceWrite,
        "Run a vendor-neutral evaluation and write bounded local results.",
        SPEC_OUTPUT
    ),
    command!(
        SuccessorCommand::Eval(EvalAction::Harvest),
        Some("harvest"),
        WorkspaceWrite,
        "Harvest independently triaged failures into a local candidate artifact.",
        INPUT_OUTPUT
    ),
    command!(
        SuccessorCommand::Eval(EvalAction::Promote),
        Some("promote"),
        WorkspaceWrite,
        "Write a bounded, reversible improvement promotion candidate.",
        CANDIDATE_OUTPUT
    ),
    command!(
        SuccessorCommand::Eval(EvalAction::Adapter),
        Some("adapter"),
        ExternalWrite,
        "Invoke one explicitly named external evaluation provider adapter.",
        ADAPTER_OPTIONS
    ),
    command!(
        SuccessorCommand::Migrate(MigrateAction::Plan),
        Some("plan"),
        Read,
        "Plan routes, compatibility, and retirement from the migration registry.",
        REGISTRY_OPTION
    ),
    command!(
        SuccessorCommand::Migrate(MigrateAction::Apply),
        Some("apply"),
        WorkspaceWrite,
        "Apply only an explicitly accepted, context-bound migration plan.",
        MIGRATE_APPLY
    ),
    command!(
        SuccessorCommand::Migrate(MigrateAction::Verify),
        Some("verify"),
        Read,
        "Verify replacement behavior and absence of duplicate authority.",
        REGISTRY_OPTION
    ),
    command!(
        SuccessorCommand::Migrate(MigrateAction::Retire),
        Some("retire"),
        Destructive,
        "Retire named legacy surfaces only with an explicit destructive approval.",
        MIGRATE_RETIRE
    ),
];

pub fn catalog() -> &'static [CommandDescriptor] {
    CATALOG
}

pub fn descriptor_for(
    group: Group,
    subcommand: Option<&str>,
) -> Option<&'static CommandDescriptor> {
    CATALOG.iter().find(|descriptor| {
        descriptor.command.group() == group && descriptor.subcommand == subcommand
    })
}
