use super::super::command_contract::{
    CommandDescriptor, EvalAction, MigrateAction, SuccessorCommand,
};
use super::options::{
    ADAPTER_OPTIONS, CANDIDATE_OUTPUT, INPUT_OUTPUT, MIGRATE_APPLY, MIGRATE_RETIRE,
    REGISTRY_OPTION, SPEC_OPTION, SPEC_OUTPUT, TARGET_OPTION, descriptor,
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
        "Produce a read-only ProductMigration projection from the migration registry; it is not the Routine HostState apply record.",
        REGISTRY_OPTION,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Abandon),
        Some("abandon-plan"),
        EffectClass::Read,
        "Produce a read-only exact-owner plan that explicitly abandons irreconcilable legacy Routine history; it does not authorize or apply the quarantine.",
        TARGET_OPTION,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Apply),
        Some("apply"),
        EffectClass::ExternalWrite,
        "Quarantine legacy Routine HostState only from an exact immutable diagnose or explicit abandonment record, its quarantine_plan.plan_id, and any required retirement approval.",
        MIGRATE_APPLY,
    ),
    descriptor(
        SuccessorCommand::Migrate(MigrateAction::Verify),
        Some("verify"),
        EffectClass::Read,
        "Verify current inventory errors and migration-plan closure without mutation.",
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
