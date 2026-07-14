use super::super::command_contract::{
    CommandDescriptor, EvalAction, MigrateAction, SuccessorCommand,
};
use super::options::{
    ADAPTER_OPTIONS, CANDIDATE_OUTPUT, INPUT_OUTPUT, MIGRATE_APPLY, MIGRATE_RETIRE,
    REGISTRY_OPTION, SPEC_OPTION, SPEC_OUTPUT, descriptor,
};
use crate::context::EffectClass;

pub(super) const COMMANDS: &[CommandDescriptor] = &[
    descriptor(
        SuccessorCommand::Eval(EvalAction::Audit),
        Some("audit"),
        EffectClass::Read,
        "Audit an evaluation specification and dataset without mutation.",
        SPEC_OPTION,
    ),
    descriptor(
        SuccessorCommand::Eval(EvalAction::Run),
        Some("run"),
        EffectClass::WorkspaceWrite,
        "Run a vendor-neutral evaluation and write bounded local results.",
        SPEC_OUTPUT,
    ),
    descriptor(
        SuccessorCommand::Eval(EvalAction::Harvest),
        Some("harvest"),
        EffectClass::WorkspaceWrite,
        "Harvest independently triaged failures into a local candidate artifact.",
        INPUT_OUTPUT,
    ),
    descriptor(
        SuccessorCommand::Eval(EvalAction::Promote),
        Some("promote"),
        EffectClass::WorkspaceWrite,
        "Write a bounded, reversible improvement promotion candidate.",
        CANDIDATE_OUTPUT,
    ),
    descriptor(
        SuccessorCommand::Eval(EvalAction::Adapter),
        Some("adapter"),
        EffectClass::ExternalWrite,
        "Invoke one explicitly named external evaluation provider adapter.",
        ADAPTER_OPTIONS,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Plan),
        Some("plan"),
        EffectClass::Read,
        "Plan routes, compatibility, and retirement from the migration registry.",
        REGISTRY_OPTION,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Apply),
        Some("apply"),
        EffectClass::WorkspaceWrite,
        "Apply only an explicitly accepted, context-bound migration plan.",
        MIGRATE_APPLY,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Verify),
        Some("verify"),
        EffectClass::Read,
        "Verify replacement behavior and absence of duplicate authority.",
        REGISTRY_OPTION,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Retire),
        Some("retire"),
        EffectClass::Destructive,
        "Retire named legacy surfaces only with an explicit destructive approval.",
        MIGRATE_RETIRE,
    ),
];
