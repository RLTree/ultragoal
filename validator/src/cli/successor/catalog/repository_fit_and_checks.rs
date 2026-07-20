use super::super::command_contract::{
    CheckProfile, CommandDescriptor, FitAction, SuccessorCommand,
};
use super::options::{
    CLAIM_OUTPUT, FINDING_OPTION, FIT_APPLY, ROUTINE_OPTIONS, STRICT_OPTIONS, TARGET_OPTION,
    descriptor,
};
use crate::context::EffectClass;

pub(super) const COMMANDS: &[CommandDescriptor] = &[
    descriptor(
        SuccessorCommand::Fit(FitAction::Inspect),
        Some("inspect"),
        EffectClass::Read,
        "Classify a fresh, partial, compatible, conflicting, or fitted target.",
        TARGET_OPTION,
    ),
    descriptor(
        SuccessorCommand::Fit(FitAction::Plan),
        Some("plan"),
        EffectClass::Read,
        "Compute desired state, exact mutations, conflicts, rollback, and authority needs.",
        TARGET_OPTION,
    ),
    descriptor(
        SuccessorCommand::Fit(FitAction::Apply),
        Some("apply"),
        EffectClass::WorkspaceWrite,
        "Apply only an explicitly accepted, context-bound fit plan.",
        FIT_APPLY,
    ),
    descriptor(
        SuccessorCommand::Fit(FitAction::Verify),
        Some("verify"),
        EffectClass::Read,
        "Verify fitted discovery and behavior without mutation.",
        TARGET_OPTION,
    ),
    descriptor(
        SuccessorCommand::Check(CheckProfile::Routine),
        Some("routine"),
        EffectClass::WorkspaceWrite,
        "Run conservative affected validation with declared local build artifacts.",
        ROUTINE_OPTIONS,
    ),
    descriptor(
        SuccessorCommand::Check(CheckProfile::Strict),
        Some("strict"),
        EffectClass::Read,
        "Run dependency-closed validation for one named claim.",
        STRICT_OPTIONS,
    ),
    descriptor(
        SuccessorCommand::Diagnose,
        None,
        EffectClass::Read,
        "Explain cause, smallest repair, exact rerun, effect, and ceiling.",
        FINDING_OPTION,
    ),
    descriptor(
        SuccessorCommand::Prove,
        None,
        EffectClass::WorkspaceWrite,
        "Execute one claim-specific proof and independent reconciliation packet.",
        CLAIM_OUTPUT,
    ),
];
