use super::catalog::{
    ActionDefinition, ActionKind, CommandBinding, DependencyFact, DependencyStatus, FactAuthority,
};
use super::product_state::{
    AuthorityRequirement, CeilingReduction, Repair, RepairTarget, RepairTargetKind, Scope,
};
use crate::context::EffectClass;
use std::collections::BTreeSet;

const REPAIR_ID: &str = "inspect-installed-daily-driver";

pub(super) fn append_first_truth_loop_route(
    dependencies: &mut Vec<DependencyFact>,
    commands: &mut Vec<CommandBinding>,
    actions: &mut Vec<ActionDefinition>,
    reductions: &[CeilingReduction],
) {
    dependencies.push(DependencyFact {
        dependency_id: "installed-daily-driver".to_owned(),
        observation_id: "current-installed-daily-driver".to_owned(),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: Scope {
            surface: "installed-daily-driver".to_owned(),
            relative_path: None,
        },
        cause: "the installed daily-driver journey has not yet observed its target repository"
            .to_owned(),
        repair: Some(repair()),
        ceiling_reductions: reductions.to_vec(),
    });
    commands.push(CommandBinding {
        command_id: "fit-inspect".to_owned(),
        argv: vec![
            "ultragoal".to_owned(),
            "--json".to_owned(),
            "fit".to_owned(),
            "inspect".to_owned(),
        ],
        effect: EffectClass::Read,
    });
    actions.push(ActionDefinition {
        action_id: "inspect-installed-daily-driver".to_owned(),
        priority: 1_000,
        kind: ActionKind::Command,
        repair_id: REPAIR_ID.to_owned(),
        requires_dependencies: Vec::new(),
        required_capabilities: Vec::new(),
        effect: EffectClass::Read,
        authority: AuthorityRequirement::Root,
        command_id: Some("fit-inspect".to_owned()),
        authority_request: None,
        evidence_led: None,
    });
}

fn repair() -> Repair {
    Repair {
        repair_id: REPAIR_ID.to_owned(),
        target: RepairTarget {
            kind: RepairTargetKind::Dependency,
            id: "installed-daily-driver".to_owned(),
        },
        summary: "Inspect the target repository through the installed daily-driver route"
            .to_owned(),
        effect: EffectClass::Read,
        authority: AuthorityRequirement::Root,
        rerun_command_id: "fit-inspect".to_owned(),
        authority_decision: None,
        invalidates_evidence: BTreeSet::from(["installed-daily-driver".to_owned()]),
        projected_ceiling_after_reverification: Vec::new(),
    }
}
