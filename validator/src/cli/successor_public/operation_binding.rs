//! Exact public-operation authority for the supported successor dispatcher.
//!
//! Catalog presence and compile visibility describe compatibility. Only this
//! private table can attest that a command/effect pair reaches one supported
//! production handler and consumes its named API family.

use crate::cli::successor::{
    CheckProfile, EffectClass, FitAction, Group, InspectTarget, ObserveAction, ParsedInvocation,
    SuccessorCommand, catalog,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicOperation {
    StrictCheck,
    RoutineCheck,
    ContextInspection,
    CapabilityInspection,
    InventoryInspection,
    StateInspection,
    NextAction,
    FitInspection,
    FitPlanning,
    FitApply,
    FitVerification,
    Diagnosis,
    ObservabilityQuery,
}

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

pub(crate) fn active_api_identifiers() -> BTreeSet<&'static str> {
    BINDINGS
        .iter()
        .flat_map(|binding| binding.apis)
        .copied()
        .collect()
}

pub(crate) fn active_command_groups() -> BTreeSet<&'static str> {
    let represented = BINDINGS
        .iter()
        .map(|binding| binding.command.group())
        .collect::<BTreeSet<Group>>();
    represented
        .into_iter()
        .filter(|group| {
            catalog()
                .iter()
                .filter(|descriptor| descriptor.command.group() == *group)
                .all(|descriptor| bind_command(descriptor.command, descriptor.effect).is_some())
        })
        .map(Group::as_str)
        .collect()
}

#[cfg(test)]
#[path = "operation_binding_tests.rs"]
mod tests;
