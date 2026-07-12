use crate::durable_support::*;
use crate::orchestration::*;
use crate::support::*;
use std::collections::BTreeSet;

fn request(class: EffectClass, target: &str) -> EffectRequest {
    EffectRequest {
        lease_id: "lease-001".to_owned(),
        binding: binding(),
        effect: effect(class, target),
        operation_id: "operation-001".to_owned(),
        payload_digest: digest('e'),
    }
}

#[test]
fn inspect_plan_recovery_and_replay_are_zero_effect() {
    let (mut engine, calls) = engine();
    engine.grant_lease(1, lease()).unwrap();
    let _ = engine.events();
    let _ = engine.plan().unwrap();
    let _ = engine.recovery_report(&binding(), 2, &BTreeSet::from(["worker-a".to_owned()]));
    let (sink, replay_calls) = CountingSink::new();
    let _ = Orchestrator::restart(
        graph_one(),
        policy(),
        binding(),
        root(),
        engine.event_log(),
        sink,
    )
    .unwrap();
    assert_eq!(calls.get(), 0);
    assert_eq!(replay_calls.get(), 0);
}

#[test]
fn authorized_effect_runs_once_through_injected_sink() {
    let (mut engine, calls, _journal) = durable_engine("effect-authorized");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine
        .apply_effect(
            3,
            request(EffectClass::WorkspaceWrite, "leased-source/node_a"),
        )
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert!(matches!(
        engine.events().last().unwrap().event,
        EventKind::EffectApplied { .. }
    ));
}

#[test]
fn effect_escalation_is_denied_before_sink() {
    let (mut engine, calls, _journal) = durable_engine("effect-escalation");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    assert_eq!(
        engine
            .apply_effect(3, request(EffectClass::Network, "focused-test"))
            .unwrap_err(),
        OrchestrationError::EffectDenied
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(
        engine
            .apply_effect(
                1,
                request(EffectClass::WorkspaceWrite, "leased-source/node_a"),
            )
            .unwrap_err(),
        OrchestrationError::EffectDenied
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn worker_cannot_mutate_after_submitting_result() {
    let (mut engine, calls, _journal) = durable_engine("effect-after-submit");
    let lease = lease();
    let package = package("node-a", &[], "node_a");
    let result = result_for(&lease, &package);
    engine.grant_lease(1, lease).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.submit_structural(3, "lease-001", &result).unwrap();
    assert_eq!(
        engine
            .apply_effect(
                4,
                request(EffectClass::WorkspaceWrite, "leased-source/node_a")
            )
            .unwrap_err(),
        OrchestrationError::EffectDenied
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn stale_effect_binding_is_denied_before_sink() {
    let (mut engine, calls, _journal) = durable_engine("effect-stale-binding");
    engine.grant_lease(1, lease()).unwrap();
    let mut stale = request(EffectClass::WorkspaceWrite, "leased-source/node_a");
    stale.binding = Binding::new(&digest('f'), &digest('e')).unwrap();
    assert_eq!(
        engine.apply_effect(2, stale).unwrap_err(),
        OrchestrationError::StaleBinding
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn expired_or_nonmonotonic_effect_is_denied_before_sink() {
    let (mut engine, calls, _journal) = durable_engine("effect-expired");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    assert_eq!(
        engine
            .apply_effect(
                21,
                request(EffectClass::WorkspaceWrite, "leased-source/node_a"),
            )
            .unwrap_err(),
        OrchestrationError::EffectDenied
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn interrupted_root_denies_effect_before_sink() {
    let (mut engine, calls, _journal) = durable_engine("effect-interrupted");
    engine.grant_lease(1, lease()).unwrap();
    engine.start(2, "lease-001").unwrap();
    engine.interrupt_root(3).unwrap();
    assert_eq!(
        engine
            .apply_effect(
                4,
                request(EffectClass::WorkspaceWrite, "leased-source/node_a"),
            )
            .unwrap_err(),
        OrchestrationError::EffectDenied
    );
    assert_eq!(calls.get(), 0);
}

fn scope_with_effect(stem: &str, class: EffectClass, target: &str) -> OwnedScope {
    let mut owned = scope(stem);
    owned.effects = [effect(class, target)].into();
    owned
}

#[test]
fn effect_aliases_conflict_within_one_scope_and_across_two_leases() {
    for class in [
        EffectClass::WorkspaceWrite,
        EffectClass::FixtureWrite,
        EffectClass::Network,
        EffectClass::ExternalWrite,
    ] {
        let lower = effect(class, "service/path");
        let mixed = effect(class, "Service/Path/Child");
        let mut duplicated = scope("node_a");
        duplicated.effects = [lower.clone(), mixed.clone()].into();
        assert_eq!(
            duplicated.validate().unwrap_err(),
            OrchestrationError::DuplicateOutput
        );

        let mut expanded = policy();
        expanded.allowed_effects.extend([lower, mixed]);
        let mut registry = LeaseRegistry::default();
        registry
            .grant(
                lease_with_scope(
                    "lease-001",
                    "node-a",
                    "worker-a",
                    scope_with_effect("node_a", class, "service/path"),
                ),
                &expanded,
                &binding(),
            )
            .unwrap();
        assert_eq!(
            registry
                .grant(
                    lease_with_scope(
                        "lease-002",
                        "node-b",
                        "worker-b",
                        scope_with_effect("node_b", class, "Service/Path/Child"),
                    ),
                    &expanded,
                    &binding(),
                )
                .unwrap_err(),
            OrchestrationError::LeaseConflict
        );
    }
}

#[test]
fn effect_authorization_stays_exact_and_non_aliases_remain_disjoint() {
    let mut exact = policy();
    exact
        .allowed_effects
        .insert(effect(EffectClass::WorkspaceWrite, "Service/Path"));
    assert_eq!(
        lease_with_scope(
            "lease-001",
            "node-a",
            "worker-a",
            scope_with_effect("node_a", EffectClass::WorkspaceWrite, "service/path"),
        )
        .validate(&exact)
        .unwrap_err(),
        OrchestrationError::UnknownScope
    );

    let mut expanded = policy();
    expanded.allowed_effects.extend([
        effect(EffectClass::WorkspaceWrite, "service/api"),
        effect(EffectClass::WorkspaceWrite, "service/apiv2"),
        effect(EffectClass::FixtureWrite, "service/api"),
    ]);
    let mut registry = LeaseRegistry::default();
    for (lease_id, node, owner, class, target) in [
        (
            "lease-001",
            "node-a",
            "worker-a",
            EffectClass::WorkspaceWrite,
            "service/api",
        ),
        (
            "lease-002",
            "node-b",
            "worker-b",
            EffectClass::WorkspaceWrite,
            "service/apiv2",
        ),
        (
            "lease-003",
            "node-c",
            "worker-c",
            EffectClass::FixtureWrite,
            "service/api",
        ),
    ] {
        registry
            .grant(
                lease_with_scope(
                    lease_id,
                    node,
                    owner,
                    scope_with_effect(node, class, target),
                ),
                &expanded,
                &binding(),
            )
            .unwrap();
    }
}
