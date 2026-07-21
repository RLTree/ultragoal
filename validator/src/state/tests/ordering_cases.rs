use super::fixture::*;
use crate::context::EffectClass;
use crate::state::catalog::{
    ActionDefinition, ActionKind, ActionPriorityClass, DependencyFact, DependencyStatus,
    EvidenceLedActionBinding, FactAuthority,
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
            evidence_led: None,
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

#[test]
fn evidence_class_outranks_numeric_priority_without_bypassing_dependencies() {
    let mut spec = spec();
    spec.dependencies = vec![missing("loop"), missing("speculation")];
    let mut speculation = command("speculative-action", "repair-speculation", 1);
    speculation.evidence_led = Some(binding(ActionPriorityClass::Speculative));
    let mut loop_action = command("loop-action", "repair-loop", 999);
    loop_action.evidence_led = Some(EvidenceLedActionBinding {
        class: ActionPriorityClass::ActiveTruthLoopTransition,
        brief_digest: brief_digest(),
        transition_id: Some("transition-2".to_owned()),
        transition_order: Some(2),
    });
    spec.actions = vec![speculation, loop_action];
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().action_id, "loop-action");
    assert_eq!(
        state.next_action().priority_class,
        Some(ActionPriorityClass::ActiveTruthLoopTransition)
    );
}

#[test]
fn earliest_active_truth_loop_transition_wins_deterministically() {
    let mut spec = spec();
    spec.dependencies = vec![missing("first"), missing("second")];
    let mut second = command("second-action", "repair-second", 1);
    second.evidence_led = Some(EvidenceLedActionBinding {
        class: ActionPriorityClass::ActiveTruthLoopTransition,
        brief_digest: brief_digest(),
        transition_id: Some("transition-2".to_owned()),
        transition_order: Some(2),
    });
    let mut first = command("first-action", "repair-first", 999);
    first.evidence_led = Some(EvidenceLedActionBinding {
        class: ActionPriorityClass::ActiveTruthLoopTransition,
        brief_digest: brief_digest(),
        transition_id: Some("transition-1".to_owned()),
        transition_order: Some(1),
    });
    spec.actions = vec![second, first];
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().action_id, "first-action");
    assert_eq!(
        state.next_action().active_transition.as_deref(),
        Some("transition-1")
    );
}

fn binding(class: ActionPriorityClass) -> EvidenceLedActionBinding {
    EvidenceLedActionBinding {
        class,
        brief_digest: brief_digest(),
        transition_id: None,
        transition_order: None,
    }
}

fn brief_digest() -> String {
    format!("sha256:{}", "1".repeat(64))
}
