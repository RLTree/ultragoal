use super::fixture::*;
use crate::context::EffectClass;
use crate::state::catalog::{
    ActionDefinition, ActionKind, DependencyFact, DependencyStatus, FactAuthority,
};
use crate::state::engine::derive_bound;
use crate::state::product_state::{AuthorityRequest, AuthorityRequirement, NextActionKind};

fn missing(id: &str) -> DependencyFact {
    DependencyFact {
        dependency_id: id.to_owned(),
        observation_id: format!("observe-{id}"),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: scope("runtime"),
        cause: format!("{id} is missing"),
        repair: Some(repair(
            &format!("repair-{id}"),
            AuthorityRequirement::None,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&["runtime"]),
    }
}

#[test]
fn next_skips_an_action_until_all_of_its_dependencies_are_satisfied() {
    let mut spec = spec();
    spec.dependencies = vec![missing("prerequisite"), missing("target")];
    let mut target = command("repair-target-now", "repair-target", 1);
    target.requires_dependencies = vec!["prerequisite".to_owned()];
    spec.actions = vec![
        target,
        command("repair-prerequisite", "repair-prerequisite", 5),
    ];
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().action_id, "repair-prerequisite");
}

#[test]
fn a_blocked_fact_cannot_route_to_a_command_even_if_the_command_has_higher_priority() {
    let mut spec = spec();
    let decision = AuthorityRequest {
        target: "release-decision".to_owned(),
        consequence: "Release remains withheld".to_owned(),
        reversible: true,
        accepted_loss_required: None,
    };
    let mut root_repair = repair(
        "request-root-decision",
        AuthorityRequirement::Root,
        EffectClass::Read,
    );
    root_repair.authority_decision = Some(decision.clone());
    spec.dependencies.push(DependencyFact {
        dependency_id: "root-decision".to_owned(),
        observation_id: "observe-decision".to_owned(),
        status: DependencyStatus::BlockedAuthority,
        authority: FactAuthority::ExternalAuthority,
        scope: scope("release"),
        cause: "root decision is required".to_owned(),
        repair: Some(root_repair),
        ceiling_reductions: reduction(&["product"]),
    });
    let mut command_route = command("unsafe-command-route", "request-root-decision", 1);
    command_route.authority = AuthorityRequirement::Root;
    spec.actions = vec![
        command_route,
        ActionDefinition {
            action_id: "request-root".to_owned(),
            priority: 2,
            kind: ActionKind::AuthorityRequest,
            repair_id: "request-root-decision".to_owned(),
            requires_dependencies: Vec::new(),
            required_capabilities: Vec::new(),
            effect: EffectClass::Read,
            authority: AuthorityRequirement::Root,
            command_id: None,
            authority_request: Some(decision),
        },
    ];
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().kind, NextActionKind::AuthorityRequest);
    assert_eq!(state.next_action().action_id, "request-root");
}

#[test]
fn semantically_identical_catalog_order_has_one_identity_and_one_state() {
    let mut left = spec();
    left.dependencies = vec![missing("a"), missing("b")];
    left.actions = vec![
        command("repair-b-action", "repair-b", 2),
        command("repair-a-action", "repair-a", 1),
    ];
    let mut right = left.clone();
    right.dependencies.reverse();
    right.actions.reverse();
    let left = catalog(left);
    let right = catalog(right);
    assert_eq!(left.catalog_id(), right.catalog_id());
    let left_state = derive_bound(inputs(), &left).unwrap();
    let right_state = derive_bound(inputs(), &right).unwrap();
    assert_eq!(left_state.state_id(), right_state.state_id());
    assert_eq!(left_state.next_action(), right_state.next_action());
}
