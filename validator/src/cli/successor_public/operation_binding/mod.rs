//! Exact public-operation authority for the supported successor dispatcher.

use crate::cli::successor::command_contract::{EvalAction, MigrateAction};
use crate::cli::successor::{
    CheckProfile, EffectClass, FitAction, Group, InspectTarget, ObserveAction, ParsedInvocation,
    SuccessorCommand, catalog,
};
use std::collections::BTreeSet;

mod migration_plan;
mod package;
mod public_operation;

pub(crate) use public_operation::PublicOperation;

#[derive(Clone, Copy)]
struct Binding {
    operation: PublicOperation,
    command: SuccessorCommand,
    effect: EffectClass,
    apis: &'static [&'static str],
}

const CONTEXT: &[&str] = &["LiveContext::build", "EffectClass"];
const CONTEXT_INSPECTION: &[&str] = &["LiveContext::build", "EffectClass", "CandidateIdentity"];
const CAPABILITIES: &[&str] = &["LiveContext::build", "EffectClass", "CapabilitySet"];
const ORCHESTRATION: &[&str] = &["LiveContext::build", "EffectClass", "SchedulerFrontier"];
const INVENTORY: &[&str] = &[
    "LiveContext::build",
    "EffectClass",
    "InventoryBuilder",
    "AuthorityCatalog",
];
const STATE: &[&str] = &[
    "LiveContext::build",
    "EffectClass",
    "InventoryBuilder",
    "AuthorityCatalog",
    "Finding",
    "Repair",
    "ProductState",
    "NextAction",
    "ClaimCeiling",
];
const FIT_INSPECT: &[&str] = &["LiveContext::build", "EffectClass", "FitInspection"];
const FIT_PLAN: &[&str] = &["LiveContext::build", "EffectClass", "FitPlan"];
const FIT_APPLY: &[&str] = &[
    "LiveContext::build",
    "FitPlan",
    "Mutation",
    "Ownership",
    "RollbackPlan",
];
const FIT_VERIFY: &[&str] = &["LiveContext::build", "EffectClass", "FitVerification"];
const ROUTINE: &[&str] = &[
    "LiveContext::build",
    "EffectClass",
    "ImpactGraph",
    "AffectedSet",
    "ReuseDecision",
    "CoverageDimensions",
];
const OBSERVABILITY: &[&str] = &[
    "LiveContext::build",
    "EffectClass",
    "SemanticEvent",
    "EventStore",
    "EventQuery",
    "CausalExplanation",
];
const EVALUATION_AUDIT: &[&str] = &[
    "LiveContext::build",
    "EffectClass",
    "EvaluationSpec",
    "TaskAudit",
];
const EVALUATION_RUN: &[&str] = &["EvalRunUnsupportedCapability"];
const BINDINGS: &[Binding] = &[
    binding(
        PublicOperation::StrictCheck,
        SuccessorCommand::Check(CheckProfile::Strict),
        EffectClass::Read,
        CONTEXT,
    ),
    binding(
        PublicOperation::RoutineCheck,
        SuccessorCommand::Check(CheckProfile::Routine),
        EffectClass::WorkspaceWrite,
        ROUTINE,
    ),
    binding(
        PublicOperation::ContextInspection,
        SuccessorCommand::Inspect(InspectTarget::Context),
        EffectClass::Read,
        CONTEXT_INSPECTION,
    ),
    binding(
        PublicOperation::CapabilityInspection,
        SuccessorCommand::Inspect(InspectTarget::Capabilities),
        EffectClass::Read,
        CAPABILITIES,
    ),
    binding(
        PublicOperation::OrchestrationInspection,
        SuccessorCommand::Inspect(InspectTarget::Orchestration),
        EffectClass::Read,
        ORCHESTRATION,
    ),
    binding(
        PublicOperation::InventoryInspection,
        SuccessorCommand::Inspect(InspectTarget::Inventory),
        EffectClass::Read,
        INVENTORY,
    ),
    binding(
        PublicOperation::StateInspection,
        SuccessorCommand::Inspect(InspectTarget::Summary),
        EffectClass::Read,
        STATE,
    ),
    binding(
        PublicOperation::StateInspection,
        SuccessorCommand::Inspect(InspectTarget::Findings),
        EffectClass::Read,
        STATE,
    ),
    binding(
        PublicOperation::StateInspection,
        SuccessorCommand::Inspect(InspectTarget::Claims),
        EffectClass::Read,
        STATE,
    ),
    binding(
        PublicOperation::NextAction,
        SuccessorCommand::Next,
        EffectClass::Read,
        STATE,
    ),
    binding(
        PublicOperation::FitInspection,
        SuccessorCommand::Fit(FitAction::Inspect),
        EffectClass::Read,
        FIT_INSPECT,
    ),
    binding(
        PublicOperation::FitPlanning,
        SuccessorCommand::Fit(FitAction::Plan),
        EffectClass::Read,
        FIT_PLAN,
    ),
    binding(
        PublicOperation::FitApply,
        SuccessorCommand::Fit(FitAction::Apply),
        EffectClass::WorkspaceWrite,
        FIT_APPLY,
    ),
    binding(
        PublicOperation::FitVerification,
        SuccessorCommand::Fit(FitAction::Verify),
        EffectClass::Read,
        FIT_VERIFY,
    ),
    binding(
        PublicOperation::Diagnosis,
        SuccessorCommand::Diagnose,
        EffectClass::Read,
        OBSERVABILITY,
    ),
    binding(
        PublicOperation::ObservabilityQuery,
        SuccessorCommand::Observe(ObserveAction::Query),
        EffectClass::Read,
        OBSERVABILITY,
    ),
    binding(
        PublicOperation::EvaluationAudit,
        SuccessorCommand::Eval(EvalAction::Audit),
        EffectClass::Read,
        EVALUATION_AUDIT,
    ),
    binding(
        PublicOperation::EvaluationRun,
        SuccessorCommand::Eval(EvalAction::Run),
        EffectClass::WorkspaceWrite,
        EVALUATION_RUN,
    ),
    binding(
        PublicOperation::MigrationPlan,
        SuccessorCommand::Migrate(MigrateAction::Plan),
        EffectClass::Read,
        migration_plan::APIS,
    ),
    package::BUILD,
    package::INSTALL_TEST,
    package::INVENTORY,
];

const fn binding(
    operation: PublicOperation,
    command: SuccessorCommand,
    effect: EffectClass,
    apis: &'static [&'static str],
) -> Binding {
    Binding {
        operation,
        command,
        effect,
        apis,
    }
}

pub(crate) fn bind(invocation: &ParsedInvocation) -> Option<PublicOperation> {
    bind_command(invocation.command, invocation.effect)
}

fn bind_command(command: SuccessorCommand, effect: EffectClass) -> Option<PublicOperation> {
    BINDINGS
        .iter()
        .find(|binding| binding.command == command && binding.effect == effect)
        .map(|binding| binding.operation)
}

include!("api_identifiers.rs");

include!("groups.rs");

#[cfg(test)]
mod tests;
