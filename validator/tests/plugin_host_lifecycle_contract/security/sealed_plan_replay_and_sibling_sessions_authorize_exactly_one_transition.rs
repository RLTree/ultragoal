#[test]
fn sealed_plan_replay_and_sibling_sessions_authorize_exactly_one_transition() {
    let fixture = Fixture::new("session-replay");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let (mut owner, host) = fixture.session(&bundle, plan.clone());
    let (mut sibling, _) = fixture.session(&bundle, plan);
    let before = fixture.tree();
    assert_eq!(
        owner.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(fixture.tree(), before);
    owner.apply_confined(&empty).unwrap();
    let after_owner = fixture.tree();
    let mut reader = Reader::complete(&bundle, &host, owner.binding());
    assert_eq!(
        owner.capture_and_verify(&mut reader).unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(
        sibling.apply_confined(&empty).unwrap_err().id(),
        HostLifecycleErrorId::LifecycleRejected
    );
    assert_eq!(
        owner.apply_confined(&empty).unwrap_err().id(),
        HostLifecycleErrorId::SessionStateRejected
    );
    assert_eq!(
        sibling.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    let request = owner.take_external_effect_request().unwrap();
    assert_eq!(
        owner.capture_and_verify(&mut reader).unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(
        owner.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectReplayed
    );
    let prepared = owner.consume_external_effect_request(request).unwrap();
    assert_eq!(
        owner.capture_and_verify(&mut reader).unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectNotEligible
    );
    assert_eq!(prepared.into_plan().unwrap().commands().len(), 1);
    assert!(owner.capture_and_verify(&mut reader).is_ok());
    assert_eq!(fixture.tree(), after_owner);
}

#[test]
fn independently_issued_identical_sessions_have_unique_requests_and_refuse_cross_session_use() {
    let fixture = Fixture::new("independent-session-crossing");
    let bundle = fixture.bundle("0.0.12");
    let empty = LifecycleState::default();
    let first_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    let second_plan = lifecycle(
        &empty,
        request(
            LifecycleIntent::FreshInstall,
            Some(bundle.authority.clone()),
            None,
            None,
            true,
            false,
        ),
    );
    assert_eq!(first_plan.plan_id, second_plan.plan_id);
    assert_ne!(
        first_plan, second_plan,
        "sealed issuances unexpectedly equal"
    );
    let (mut first, _) = fixture.session(&bundle, first_plan);
    let (mut second, _) = fixture.session(&bundle, second_plan);

    first.apply_confined(&empty).unwrap();
    fixture.replace(
        "installed/harness-ultragoal.hugpkg",
        Some(&bundle.authority.package_sha256),
        None,
    );
    fixture.replace(
        "cache/harness-ultragoal.hugpkg",
        Some(&bundle.authority.package_sha256),
        None,
    );
    second.apply_confined(&empty).unwrap();

    let first_request = first.take_external_effect_request().unwrap();
    let second_request = second.take_external_effect_request().unwrap();
    assert_ne!(
        first_request.session_issuance_sha256(),
        second_request.session_issuance_sha256()
    );
    assert_ne!(
        first_request.request_sha256(),
        second_request.request_sha256()
    );
    let before_cross = fixture.tree();
    assert_eq!(
        first
            .consume_external_effect_request(second_request)
            .unwrap_err()
            .id(),
        HostLifecycleErrorId::ExternalEffectSessionMismatch
    );
    assert_eq!(fixture.tree(), before_cross);
    let prepared = first
        .consume_external_effect_request(first_request)
        .unwrap();
    assert_eq!(prepared.into_plan().unwrap().commands().len(), 1);
    assert_eq!(
        second.take_external_effect_request().unwrap_err().id(),
        HostLifecycleErrorId::ExternalEffectReplayed
    );
    assert_eq!(fixture.tree(), before_cross);
}
