use super::catalog::{ActionDefinition, ActionKind, CommandBinding, DependencyStatus};
use super::product_state::{
    AuthorityRequirement, Finding, NextAction, NextActionKind, NoLegalRoute,
};
use crate::context::EffectClass;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn select(
    commands: &[CommandBinding],
    actions: &[ActionDefinition],
    findings: &[Finding],
    dependencies: &BTreeMap<String, Option<DependencyStatus>>,
    capabilities: &BTreeMap<String, bool>,
    fail_closed: bool,
) -> NextAction {
    if findings.is_empty() {
        return no_op();
    }
    if fail_closed {
        return no_route(preferred_finding(findings), "fatal-input-fail-closed");
    }
    let repairs = findings
        .iter()
        .map(|finding| finding.repair.repair_id.as_str())
        .collect::<BTreeSet<_>>();
    let blocked_repairs = findings
        .iter()
        .filter(|finding| finding.severity == super::product_state::FindingSeverity::Blocked)
        .map(|finding| finding.repair.repair_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut legal = actions
        .iter()
        .filter(|action| repairs.contains(action.repair_id.as_str()))
        .filter(|action| {
            !blocked_repairs.contains(action.repair_id.as_str())
                || action.kind == ActionKind::AuthorityRequest
        })
        .filter(|action| {
            action
                .requires_dependencies
                .iter()
                .all(|id| dependencies.get(id) == Some(&Some(DependencyStatus::Satisfied)))
        })
        .filter(|action| {
            action
                .required_capabilities
                .iter()
                .all(|name| capabilities.get(name).copied().unwrap_or(false))
        })
        .filter(|action| shape_is_valid(action))
        .collect::<Vec<_>>();
    legal.sort_by(|a, b| (a.priority, &a.action_id).cmp(&(b.priority, &b.action_id)));
    if let Some(action) = legal.first()
        && let Some(next) = from_definition(action, commands)
    {
        return next;
    }
    no_route(
        preferred_finding(findings),
        "no-dependency-closed-catalog-route",
    )
}

pub(crate) fn shape_is_valid(action: &ActionDefinition) -> bool {
    match action.kind {
        ActionKind::Command => action.command_id.is_some() && action.authority_request.is_none(),
        ActionKind::AuthorityRequest => {
            action.command_id.is_none() && action.authority_request.is_some()
        }
    }
}

fn from_definition(action: &ActionDefinition, commands: &[CommandBinding]) -> Option<NextAction> {
    let command = action
        .command_id
        .as_deref()
        .and_then(|id| commands.iter().find(|item| item.command_id == id));
    if action.kind == ActionKind::Command && command.is_none() {
        return None;
    }
    Some(NextAction {
        kind: match action.kind {
            ActionKind::Command => NextActionKind::Command,
            ActionKind::AuthorityRequest => NextActionKind::AuthorityRequest,
        },
        action_id: action.action_id.clone(),
        priority: action.priority,
        repair_id: Some(action.repair_id.clone()),
        effect: action.effect,
        authority: action.authority,
        command_id: action.command_id.clone(),
        exact_command: command.map(|item| item.argv.clone()),
        authority_request: action.authority_request.clone(),
        no_legal_route: None,
        selection_rule: "lowest-priority-number-then-lexical-action-id",
    })
}

fn no_op() -> NextAction {
    NextAction {
        kind: NextActionKind::NoOp,
        action_id: "no-op".to_owned(),
        priority: u32::MAX,
        repair_id: None,
        effect: EffectClass::Read,
        authority: AuthorityRequirement::None,
        command_id: None,
        exact_command: None,
        authority_request: None,
        no_legal_route: None,
        selection_rule: "no-actionable-findings",
    }
}

fn no_route(finding: &Finding, rule: &'static str) -> NextAction {
    NextAction {
        kind: NextActionKind::NoLegalRoute,
        action_id: format!("no-legal-route:{}", finding.repair.repair_id),
        priority: u32::MAX - 1,
        repair_id: Some(finding.repair.repair_id.clone()),
        effect: finding.repair.effect,
        authority: finding.repair.authority,
        command_id: None,
        exact_command: None,
        authority_request: None,
        no_legal_route: Some(NoLegalRoute {
            target: finding.repair.target.clone(),
            required_authority: finding.repair.authority,
            required_decision: finding.repair.authority_decision.clone(),
            reason: "The dependency/action catalog contains no legal route for this repair"
                .to_owned(),
        }),
        selection_rule: rule,
    }
}

fn preferred_finding(findings: &[Finding]) -> &Finding {
    findings
        .iter()
        .find(|finding| finding.code == "invalid-state-policy")
        .or_else(|| {
            findings.iter().min_by_key(|finding| {
                (
                    Reverse(finding.authority),
                    Reverse(finding.effect),
                    &finding.code,
                    &finding.finding_id,
                )
            })
        })
        .expect("nonempty findings")
}
