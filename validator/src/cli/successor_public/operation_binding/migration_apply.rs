use super::{Binding, PublicOperation, binding};
use crate::cli::successor::command_contract::MigrateAction;
use crate::cli::successor::{EffectClass, SuccessorCommand};

const APIS: &[&str] = &[
    "MigrateAction::Apply",
    "RoutineStateQuarantinePlan",
    "ImmutableAcceptedPlan",
    "RoutineStateQuarantineApplyOutcome",
];

pub(super) const BINDING: Binding = binding(
    PublicOperation::MigrationApply,
    SuccessorCommand::Migrate(MigrateAction::Apply),
    EffectClass::ExternalWrite,
    APIS,
);
