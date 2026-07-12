use super::fixture::*;
use crate::context::EffectClass;
use crate::inventory::FindingSeverity as InventorySeverity;
use crate::state::catalog::{DependencyFact, DependencyStatus, FactAuthority};
use crate::state::engine::derive_bound;
use crate::state::snapshot::InventoryObservation;
use crate::state::types::{AuthorityRequirement, FindingSource, NextActionKind};
use std::collections::BTreeSet;

fn missing(
    dependency_id: &str,
    observation_id: &str,
    repair_id: &str,
    authority: AuthorityRequirement,
    effect: EffectClass,
    dimensions: &[&str],
) -> DependencyFact {
    DependencyFact {
        dependency_id: dependency_id.to_owned(),
        observation_id: observation_id.to_owned(),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: scope("runtime"),
        cause: format!("{observation_id} observed the dependency missing"),
        repair: Some(repair(repair_id, authority, effect)),
        ceiling_reductions: reduction(dimensions),
    }
}

#[test]
fn fatal_input_with_an_otherwise_legal_command_never_operates() {
    let mut inputs = inputs();
    inputs.inventory_findings.push(InventoryObservation {
        code: "unmapped-live-finding".to_owned(),
        severity: InventorySeverity::Error,
        entry_id: Some("entry".to_owned()),
        relative_path: Some("source/file.rs".to_owned()),
        cause: "live authority finding has no policy".to_owned(),
    });
    let mut spec = spec();
    spec.dependencies.push(missing(
        "dep-a",
        "probe-a",
        "repair-dep-a",
        AuthorityRequirement::None,
        EffectClass::Read,
        &["runtime"],
    ));
    spec.actions
        .push(command("legal-command", "repair-dep-a", 1));
    assert_eq!(
        derive_bound(inputs, &catalog(spec)),
        Err(crate::state::types::StateError::InvalidCatalog(
            "policy-inventory-impact-missing".to_owned()
        ))
    );
}

#[test]
fn absent_external_route_preserves_external_authority_and_decision() {
    let mut spec = spec();
    let fact = missing(
        "marketplace-access",
        "probe-marketplace",
        "request-marketplace",
        AuthorityRequirement::External,
        EffectClass::Read,
        &["product"],
    );
    let expected = fact.repair.as_ref().unwrap().authority_decision.clone();
    spec.dependencies.push(fact);
    spec.dependencies.push(missing(
        "root-repair",
        "probe-root",
        "root-route",
        AuthorityRequirement::Root,
        EffectClass::Read,
        &["runtime"],
    ));
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().kind, NextActionKind::NoLegalRoute);
    assert_eq!(
        state.next_action().authority,
        AuthorityRequirement::External
    );
    assert_eq!(
        state
            .next_action()
            .no_legal_route
            .as_ref()
            .unwrap()
            .required_decision,
        expected
    );
}

#[test]
fn absent_destructive_route_preserves_loss_and_human_authority() {
    let mut spec = spec();
    let fact = missing(
        "destructive-cleanup",
        "probe-cleanup",
        "approve-destructive-cleanup",
        AuthorityRequirement::HumanDestructive,
        EffectClass::Destructive,
        &["product"],
    );
    let expected = fact.repair.as_ref().unwrap().authority_decision.clone();
    spec.dependencies.push(fact);
    spec.dependencies.push(missing(
        "root-repair",
        "probe-root",
        "root-route",
        AuthorityRequirement::Root,
        EffectClass::Read,
        &["runtime"],
    ));
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().kind, NextActionKind::NoLegalRoute);
    assert_eq!(
        state.next_action().authority,
        AuthorityRequirement::HumanDestructive
    );
    assert_eq!(
        state
            .next_action()
            .no_legal_route
            .as_ref()
            .unwrap()
            .required_decision,
        expected
    );
    assert!(expected.unwrap().accepted_loss_required.is_some());
}

#[test]
fn same_status_observations_are_all_preserved_and_order_independent() {
    let mut left = spec();
    let mut first = missing(
        "shared-dependency",
        "probe-alpha",
        "repair-alpha",
        AuthorityRequirement::None,
        EffectClass::Read,
        &["runtime"],
    );
    first.authority = FactAuthority::LiveContext;
    let mut second = missing(
        "shared-dependency",
        "probe-beta",
        "repair-beta",
        AuthorityRequirement::Root,
        EffectClass::Read,
        &["product"],
    );
    second.authority = FactAuthority::AuthorityCatalog;
    second.scope = scope("product");
    left.dependencies = vec![first, second];
    let mut right = left.clone();
    right.dependencies.reverse();
    let left = catalog(left);
    let right = catalog(right);
    assert_eq!(left.catalog_id(), right.catalog_id());
    let first_state = derive_bound(inputs(), &left).unwrap();
    let second_state = derive_bound(inputs(), &right).unwrap();
    assert_eq!(first_state.state_id(), second_state.state_id());
    let observations = first_state
        .findings()
        .iter()
        .filter_map(|finding| match &finding.source {
            FindingSource::DependencyCatalog { observation_id, .. } => Some(observation_id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        observations,
        BTreeSet::from(["probe-alpha".to_owned(), "probe-beta".to_owned()])
    );
    let ceiling = &first_state.claim_ceilings()[0];
    assert!(!ceiling.dimensions().contains("runtime"));
    assert!(!ceiling.dimensions().contains("product"));
}
