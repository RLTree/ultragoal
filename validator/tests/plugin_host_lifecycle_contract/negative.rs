use crate::distribution::{HostCapabilityDeclaration, registry_document};
use crate::host_fixture::{
    Fixture, Reader, installed, lifecycle, marketplace_plan, request, ui_document,
};
use crate::host_lifecycle::{
    HostLayer, HostLayerVerdict, HostLifecycleBindRequest, HostLifecycleErrorId,
    HostLifecyclePhase, HostLifecycleSession, HostScopeAuthority, observe_plugins_ui,
};
use crate::plugin_product::lifecycle::{LifecycleIntent, LifecycleState};

#[test]
fn unsupported_host_is_explicit_and_supplied_bytes_cannot_promote_it() {
    let fixture = Fixture::new("unsupported-host");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 2);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let host = HostCapabilityDeclaration::unavailable_codex_app(
        &fixture.root,
        &fixture.project,
        "codex-app-unavailable",
    )
    .unwrap();
    let mut session = HostLifecycleSession::bind(HostLifecycleBindRequest {
        root: fixture.confined(),
        package_plan: &bundle.plan,
        package: &bundle.snapshot,
        lifecycle: repeat.clone(),
        host,
        marketplace_plan: marketplace_plan(&bundle),
        host_scope: HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        },
    })
    .unwrap();
    session.apply_confined(&state).unwrap();
    let mut supplied = Reader {
        provenance_sha256: session.binding().binding_sha256().to_owned(),
        marketplace: Some(marketplace_plan(&bundle).replacement().to_vec()),
        ..Reader::default()
    };
    let error = session.capture_and_verify(&mut supplied).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::UnsupportedSubstitution);

    let mut empty = Reader::empty(session.binding());
    let report = session.capture_and_verify(&mut empty).unwrap();
    assert_eq!(
        report.phase(),
        HostLifecyclePhase::AwaitingSupportedHostObservation
    );
    assert_eq!(
        report.layer(HostLayer::Discovery).verdict(),
        HostLayerVerdict::Unsupported
    );
    assert!(report.identity_chain_sha256().is_none());
}

#[test]
fn stale_cache_hidden_discovery_and_observation_drift_fail_for_their_causal_class() {
    let fixture = Fixture::new("observation-negative");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 2);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();

    let mut stale = Reader::complete(&bundle, &host, session.binding());
    let mut cache: serde_json::Value =
        serde_json::from_slice(stale.cache.as_ref().unwrap()).unwrap();
    cache["candidate_id"] = serde_json::json!(
        "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    );
    stale.cache = Some(serde_json::to_vec(&cache).unwrap());
    assert_eq!(
        session.capture_and_verify(&mut stale).unwrap_err().id(),
        HostLifecycleErrorId::ObservationConflict
    );

    let mut hidden = Reader::complete(&bundle, &host, session.binding());
    hidden.registry = Some(registry_document(session.binding(), true, false).unwrap());
    assert_eq!(
        session.capture_and_verify(&mut hidden).unwrap_err().id(),
        HostLifecycleErrorId::ObservationConflict
    );

    let mut drifting = Reader::complete(&bundle, &host, session.binding());
    drifting.mutate_marketplace_after_first = true;
    assert_eq!(
        session.capture_and_verify(&mut drifting).unwrap_err().id(),
        HostLifecycleErrorId::ObservationChanged
    );
}

#[test]
fn plugins_ui_requires_one_exact_visible_identity() {
    let fixture = Fixture::new("ui-observation");
    let bundle = fixture.bundle("0.0.12");
    let host = fixture.host();
    let binding = crate::distribution::JourneyBinding::new(
        bundle.snapshot.identity().clone(),
        &host,
        "local-harness-plugins",
    )
    .unwrap();
    let bytes = ui_document(&binding);
    let observed = observe_plugins_ui(&bytes, &binding).unwrap();
    assert_eq!(observed.verdict(), HostLayerVerdict::Verified);
    assert_eq!(observed.binding_sha256(), binding.binding_sha256());

    let mut duplicate: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let row = duplicate["entries"][0].clone();
    duplicate["entries"].as_array_mut().unwrap().push(row);
    assert_eq!(
        observe_plugins_ui(&serde_json::to_vec(&duplicate).unwrap(), &binding)
            .unwrap_err()
            .id(),
        HostLifecycleErrorId::ObservationConflict
    );

    let mut hidden: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    hidden["entries"][0]["visible"] = serde_json::json!(false);
    assert_eq!(
        observe_plugins_ui(&serde_json::to_vec(&hidden).unwrap(), &binding)
            .unwrap_err()
            .id(),
        HostLifecycleErrorId::ObservationConflict
    );
}

#[test]
fn conflicting_package_and_host_effect_arguments_refuse_before_mutation() {
    let fixture = Fixture::new("binding-negative");
    let expected = fixture.bundle("0.0.12");
    let other = fixture.bundle("0.0.13");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(expected.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let host = fixture.host();
    let before = fixture.tree();
    let mismatch = HostLifecycleSession::bind(HostLifecycleBindRequest {
        root: fixture.confined(),
        package_plan: &expected.plan,
        package: &expected.snapshot,
        lifecycle: plan.clone(),
        host: host.clone(),
        marketplace_plan: marketplace_plan(&other),
        host_scope: HostScopeAuthority::Personal {
            marketplace: "local-harness-plugins".into(),
        },
    });
    assert_eq!(
        mismatch.err().unwrap().id(),
        HostLifecycleErrorId::InvalidBinding
    );
    assert_eq!(fixture.tree(), before);

    let invalid_effect = HostLifecycleSession::bind(HostLifecycleBindRequest {
        root: fixture.confined(),
        package_plan: &expected.plan,
        package: &expected.snapshot,
        lifecycle: plan,
        host: host.clone(),
        marketplace_plan: marketplace_plan(&expected),
        host_scope: HostScopeAuthority::Personal {
            marketplace: "../../attacker".into(),
        },
    });
    assert_eq!(
        invalid_effect.err().unwrap().id(),
        HostLifecycleErrorId::InvalidBinding
    );
    assert_eq!(fixture.tree(), before);
}
