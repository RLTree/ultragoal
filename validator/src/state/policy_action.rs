use super::catalog::{ActionKind, DependencyActionSpec};
use super::limits::{valid_argv, valid_id, valid_text};
use super::next;
use super::product_state::{AuthorityRequirement, Repair};
use crate::context::EffectClass;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate(
    spec: &DependencyActionSpec,
    repairs: &BTreeMap<&str, &Repair>,
    problems: &mut Vec<String>,
) {
    let dependencies = spec
        .dependencies
        .iter()
        .map(|fact| fact.dependency_id.as_str())
        .collect::<BTreeSet<_>>();
    let capabilities = spec
        .capability_requirements
        .iter()
        .map(|item| item.capability.as_str())
        .collect::<BTreeSet<_>>();
    let commands = spec
        .commands
        .iter()
        .map(|item| (item.command_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    match commands.get("inspect-json") {
        Some(command)
            if command.effect == EffectClass::Read
                && command.argv == ["ultragoal", "--json", "inspect"] => {}
        _ => problems.push("missing-canonical-inspect-json-command".to_owned()),
    }
    for command in &spec.commands {
        if !valid_id(&command.command_id) || !valid_argv(&command.argv) {
            problems.push("invalid-command-binding".to_owned());
        }
    }
    for repair in repairs.values() {
        let Some(rerun) = commands.get(repair.rerun_command_id.as_str()) else {
            problems.push("unknown-rerun-command".to_owned());
            continue;
        };
        if rerun.effect != EffectClass::Read {
            problems.push("rerun-command-is-not-read".to_owned());
        }
    }
    for action in &spec.actions {
        if !valid_id(&action.action_id)
            || !valid_id(&action.repair_id)
            || !next::shape_is_valid(action)
        {
            problems.push("invalid-action-shape".to_owned());
        }
        let Some(repair) = repairs.get(action.repair_id.as_str()) else {
            problems.push("unknown-action-repair".to_owned());
            continue;
        };
        if action.effect != repair.effect || action.authority != repair.authority {
            problems.push("action-repair-authority-mismatch".to_owned());
        }
        validate_route(action, repair, &commands, problems);
        for dependency in &action.requires_dependencies {
            if !dependencies.contains(dependency.as_str()) {
                problems.push("unknown-action-dependency".to_owned());
            }
        }
        for capability in &action.required_capabilities {
            if !capabilities.contains(capability.as_str()) {
                problems.push("uncataloged-action-capability".to_owned());
            }
        }
    }
}

fn validate_route(
    action: &super::catalog::ActionDefinition,
    repair: &Repair,
    commands: &BTreeMap<&str, &super::catalog::CommandBinding>,
    problems: &mut Vec<String>,
) {
    match action.kind {
        ActionKind::Command => {
            let Some(command) = action.command_id.as_deref().and_then(|id| commands.get(id)) else {
                problems.push("unknown-action-command".to_owned());
                return;
            };
            if command.effect != action.effect
                || matches!(
                    action.effect,
                    EffectClass::ExternalWrite | EffectClass::Destructive
                )
                || matches!(
                    action.authority,
                    AuthorityRequirement::External | AuthorityRequirement::HumanDestructive
                )
            {
                problems.push("command-effect-mismatch".to_owned());
            }
        }
        ActionKind::AuthorityRequest => {
            if action.authority_request.as_ref() != repair.authority_decision.as_ref() {
                problems.push("authority-decision-route-mismatch".to_owned());
            }
            if action.authority_request.as_ref().is_some_and(|item| {
                !valid_id(&item.target) || !valid_text(&item.consequence, false)
            }) {
                problems.push("invalid-authority-request".to_owned());
            }
        }
    }
}
