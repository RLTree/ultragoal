use super::super::command_contract::{CommandDescriptor, InspectTarget, SuccessorCommand};
use super::options::{CAPABILITIES_OPTION, descriptor};
use crate::context::EffectClass;

pub(super) const COMMANDS: &[CommandDescriptor] = &[
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Summary),
        None,
        EffectClass::Read,
        "Summarize live context, findings, capabilities, and claim ceilings.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Context),
        Some("context"),
        EffectClass::Read,
        "Inspect candidate-bound live context.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Orchestration),
        Some("orchestration"),
        EffectClass::Read,
        "Inspect the current canonical orchestration frontier without authority exposure.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Inception),
        Some("inception"),
        EffectClass::Read,
        "Inspect the current Product Success Brief and first truth loop without writes.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Inventory),
        Some("inventory"),
        EffectClass::Read,
        "Inspect semantic components and authority conflicts.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Capabilities),
        Some("capabilities"),
        EffectClass::Read,
        "Inspect exposed tools and candidate-bound agent authority availability.",
        CAPABILITIES_OPTION,
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Findings),
        Some("findings"),
        EffectClass::Read,
        "Inspect typed current findings.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Inspect(InspectTarget::Claims),
        Some("claims"),
        EffectClass::Read,
        "Inspect per-claim ceilings without promotion.",
        &[],
    ),
    descriptor(
        SuccessorCommand::Next,
        None,
        EffectClass::Read,
        "Select one deterministic legal action or authority request.",
        &[],
    ),
];
