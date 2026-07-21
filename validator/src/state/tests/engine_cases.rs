use super::fixture::*;
use crate::context::EffectClass;
use crate::state::catalog::{
    ActionDefinition, ActionKind, CapabilityRequirement, DependencyFact, DependencyStatus,
    FactAuthority, HostGoalStatus, RuntimeField, RuntimeRequirement,
};
use crate::state::ceiling::{CeilingRelation, ClaimCeiling};
use crate::state::engine::derive_bound;
use crate::state::product_state::{AuthorityRequirement, NextActionKind, ProductGoalState};

#[test]
fn no_findings_is_an_explicit_no_op_not_completion() {
    let state = derive_bound(inputs(), &catalog(spec())).unwrap();
    assert!(state.findings().is_empty());
    assert_eq!(state.next_action().kind, NextActionKind::NoOp);
    assert_eq!(state.product_goal(), ProductGoalState::NoAction);
    assert_eq!(
        state.claim_ceilings()[0].dimensions().len(),
        3,
        "no-op does not manufacture or erase proof"
    );
}

#[test]
fn contradictory_facts_are_causal_and_lower_only_mapped_dimensions() {
    let mut spec = spec();
    spec.dependencies = vec![
        DependencyFact {
            dependency_id: "dep-a".to_owned(),
            observation_id: "probe-a".to_owned(),
            status: DependencyStatus::Satisfied,
            authority: FactAuthority::DirectProbe,
            scope: scope("runtime"),
            cause: "probe observed present".to_owned(),
            repair: None,
            ceiling_reductions: reduction(&["runtime"]),
        },
        DependencyFact {
            dependency_id: "dep-a".to_owned(),
            observation_id: "probe-b".to_owned(),
            status: DependencyStatus::Missing,
            authority: FactAuthority::AuthorityCatalog,
            scope: scope("runtime"),
            cause: "catalog observed absent".to_owned(),
            repair: Some(repair(
                "repair-dep-a",
                AuthorityRequirement::None,
                EffectClass::Read,
            )),
            ceiling_reductions: reduction(&["runtime"]),
        },
    ];
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert!(
        state
            .findings()
            .iter()
            .any(|item| item.code == "contradictory-dependency")
    );
    assert!(!state.claim_ceilings()[0].dimensions().contains("runtime"));
    assert!(state.claim_ceilings()[0].dimensions().contains("source"));
}

#[test]
fn equal_priority_actions_use_lexical_id_as_a_stable_tie_breaker() {
    let mut spec = spec();
    spec.dependencies.push(DependencyFact {
        dependency_id: "dep-a".to_owned(),
        observation_id: "probe".to_owned(),
        status: DependencyStatus::Missing,
        authority: FactAuthority::DirectProbe,
        scope: scope("runtime"),
        cause: "dependency absent".to_owned(),
        repair: Some(repair(
            "repair-dep-a",
            AuthorityRequirement::None,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&["runtime"]),
    });
    spec.actions = vec![
        command("z-action", "repair-dep-a", 10),
        command("a-action", "repair-dep-a", 10),
    ];
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().action_id, "a-action");
    assert_eq!(
        state.next_action().selection_rule,
        "lowest-priority-number-then-lexical-action-id"
    );
}

#[test]
fn blocked_external_dependency_selects_one_exact_authority_request() {
    let mut spec = spec();
    spec.dependencies.push(DependencyFact {
        dependency_id: "external-access".to_owned(),
        observation_id: "probe".to_owned(),
        status: DependencyStatus::BlockedAuthority,
        authority: FactAuthority::ExternalAuthority,
        scope: scope("install"),
        cause: "marketplace access unavailable".to_owned(),
        repair: Some(repair(
            "request-marketplace",
            AuthorityRequirement::External,
            EffectClass::Read,
        )),
        ceiling_reductions: reduction(&["product"]),
    });
    spec.actions.push(ActionDefinition {
        action_id: "request-marketplace-access".to_owned(),
        priority: 1,
        kind: ActionKind::AuthorityRequest,
        repair_id: "request-marketplace".to_owned(),
        requires_dependencies: Vec::new(),
        required_capabilities: Vec::new(),
        effect: EffectClass::Read,
        authority: AuthorityRequirement::External,
        command_id: None,
        authority_request: repair(
            "request-marketplace",
            AuthorityRequirement::External,
            EffectClass::Read,
        )
        .authority_decision,
        evidence_led: None,
    });
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert_eq!(state.next_action().kind, NextActionKind::AuthorityRequest);
    assert_eq!(state.next_action().action_id, "request-marketplace-access");
    assert_eq!(state.product_goal(), ProductGoalState::AwaitingAuthority);
}

#[test]
fn stale_identities_fail_closed_and_do_not_reuse_catalog_state() {
    let mut spec = spec();
    spec.expected_context_id = "sha256:old-context".to_owned();
    spec.expected_authority_catalog_id = "sha256:old-catalog".to_owned();
    assert_eq!(
        catalog_for_codes(spec, Default::default()),
        Err(crate::state::product_state::StateError::InvalidCatalog(
            "policy-context-binding-mismatch".to_owned()
        ))
    );
}

#[test]
fn catalog_built_for_another_live_context_is_rejected() {
    let mut inputs = inputs();
    inputs.authority_catalog_context_id = "sha256:different-context".to_owned();
    assert_eq!(
        derive_bound(inputs, &catalog(spec())),
        Err(crate::state::product_state::StateError::InvalidCatalog(
            "policy-context-binding-mismatch".to_owned()
        ))
    );
}

#[test]
fn unsupported_capability_and_missing_runtime_metadata_lower_affected_dimensions() {
    let mut spec = spec();
    spec.capability_requirements.push(CapabilityRequirement {
        capability: "host-plugin-reload".to_owned(),
        scope: scope("discovery"),
        repair: repair(
            "expose-reload",
            AuthorityRequirement::Root,
            EffectClass::Read,
        ),
        ceiling_reductions: reduction(&["product"]),
    });
    spec.runtime_requirements.push(RuntimeRequirement {
        field: RuntimeField::Reasoning,
        scope: scope("runtime"),
        repair: repair(
            "expose-reasoning",
            AuthorityRequirement::Root,
            EffectClass::Read,
        ),
        ceiling_reductions: reduction(&["runtime"]),
    });
    let state = derive_bound(inputs(), &catalog(spec)).unwrap();
    assert!(
        state
            .findings()
            .iter()
            .any(|item| item.code == "unsupported-capability")
    );
    assert!(
        state
            .findings()
            .iter()
            .any(|item| item.code == "unverified-runtime-metadata")
    );
    assert_eq!(
        state.claim_ceilings()[0].dimensions(),
        &["source".to_owned()].into_iter().collect()
    );
}

#[test]
fn host_goal_state_cannot_change_product_state_identity_or_claims() {
    let first = derive_bound(inputs(), &catalog(spec())).unwrap();
    let mut changed = spec();
    changed.host_goal = crate::state::catalog::HostGoalObservation::test_exposed(
        HostGoalStatus::Complete,
        "host-api",
        "sha256:context",
    );
    let second = derive_bound(inputs(), &catalog(changed)).unwrap();
    assert_eq!(first.state_id(), second.state_id());
    assert_eq!(first.claim_ceilings(), second.claim_ceilings());
    assert_eq!(first.next_action(), second.next_action());
    assert!(!second.host_goal().authoritative_for_product_claims());
}

#[test]
fn per_claim_ceiling_order_can_be_incomparable() {
    let left = ClaimCeiling::from_dimensions(
        "CL-RUNTIME",
        vec!["source".to_owned(), "runtime".to_owned()],
    );
    let right = ClaimCeiling::from_dimensions(
        "CL-RUNTIME",
        vec!["source".to_owned(), "product".to_owned()],
    );
    assert_eq!(left.relation(&right), CeilingRelation::Incomparable);
}
