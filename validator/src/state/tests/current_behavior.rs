use super::fixture::*;
use crate::context::EffectClass;
use crate::state::catalog::{DependencyFact, DependencyStatus, FactAuthority};
use crate::state::engine::derive_bound;
use crate::state::product_state::{AuthorityRequirement, CurrentBehaviorDisposition};

#[test]
fn findings_never_collapse_to_no_change() {
    let mut current = spec();
    current.dependencies.push(DependencyFact {
        dependency_id: "remaining".to_owned(),
        observation_id: "probe-remaining".to_owned(),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: scope("source"),
        cause: "remaining semantic defect".to_owned(),
        repair: Some(repair(
            "repair-remaining",
            AuthorityRequirement::None,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&["source"]),
    });
    current
        .actions
        .push(command("repair-remaining-action", "repair-remaining", 1));
    let state = derive_bound(inputs(), &catalog(current)).unwrap();
    assert_eq!(
        state.current_behavior(),
        CurrentBehaviorDisposition::ChangeRequired
    );
    assert_ne!(
        state.current_behavior(),
        CurrentBehaviorDisposition::NoChange
    );
}

#[test]
fn partial_change_keeps_remaining_action_and_authority_is_blocked() {
    let mut current = spec();
    current.dependencies = vec![
        missing("first", "source", "repair-first"),
        missing("second", "runtime", "repair-second"),
    ];
    current.actions = vec![command("repair-first-action", "repair-first", 1)];
    let state = derive_bound(inputs(), &catalog(current)).unwrap();
    assert_eq!(
        state.next_action().repair_id.as_deref(),
        Some("repair-first")
    );
    assert_eq!(
        state.current_behavior(),
        CurrentBehaviorDisposition::PartialChange
    );

    let mut blocked = spec();
    blocked.dependencies.push(DependencyFact {
        dependency_id: "authority".to_owned(),
        observation_id: "probe-authority".to_owned(),
        status: DependencyStatus::BlockedAuthority,
        authority: FactAuthority::ExternalAuthority,
        scope: scope("install"),
        cause: "missing authority".to_owned(),
        repair: Some(repair(
            "request-authority",
            AuthorityRequirement::External,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&["product"]),
    });
    let blocked_state = derive_bound(inputs(), &catalog(blocked)).unwrap();
    assert_eq!(
        blocked_state.current_behavior(),
        CurrentBehaviorDisposition::Blocked
    );
}

fn missing(id: &str, surface: &str, repair_id: &str) -> DependencyFact {
    DependencyFact {
        dependency_id: id.to_owned(),
        observation_id: format!("probe-{id}"),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: scope(surface),
        cause: format!("{id} remains"),
        repair: Some(repair(
            repair_id,
            AuthorityRequirement::None,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&[surface]),
    }
}
