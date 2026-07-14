use super::*;

#[test]
pub(crate) fn missing_permit_missing_lease_and_time_windows_refuse_before_effect_without_consumption()
 {
    let fixture = Fixture::new("missing-permit");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("missing"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        None,
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitMissing);
    assert!(!failure.effect_started());
    let pre = failure.into_pre_effect().unwrap();
    let (request, missing, lease) = pre.into_parts();
    assert!(missing.is_none());
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    assert_eq!(snapshot(&fixture.root), before);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        lease,
        10,
    ));
    assert_eq!(outcome.status(), "applied");

    let fixture = Fixture::new("missing-lease");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            50,
            60,
            &nonce("missing-lease"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit::<LocalEffects>(
        &context,
        request,
        Some(permit),
        None,
        50,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyLeaseInvalid);
    assert!(!failure.effect_started());
    let (request, permit, missing) = failure.into_pre_effect().unwrap().into_parts();
    assert!(missing.is_none());
    assert_eq!(snapshot(&fixture.root), before);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        permit,
        Some(lease),
        50,
    ));
    assert_eq!(outcome.status(), "applied");

    let fixture = Fixture::new("not-yet-valid");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            100,
            110,
            &nonce("not-yet-valid"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        99,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitExpired);
    assert!(!failure.effect_started());
    let (request, permit, lease) = failure.into_pre_effect().unwrap().into_parts();
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context, request, permit, lease, 100,
    ));
    assert_eq!(outcome.status(), "applied");

    let fixture = Fixture::new("expired");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            200,
            210,
            &nonce("expired"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        211,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyPermitExpired);
    assert!(!failure.effect_started());
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
pub(crate) fn permit_and_mutation_lease_must_share_the_exact_request_and_authority_instance() {
    let fixture = Fixture::new("lease-mismatch");
    let context = fixture.context();
    let request = fixture.request(&context);
    let other_request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("lease-match"),
        )
        .unwrap();
    let other_authority = new_authority();
    let (_other_permit, other_lease) = other_authority
        .issue(
            &context,
            &other_request,
            fixture.effects(&other_request),
            10,
            20,
            &nonce("lease-mismatch"),
        )
        .unwrap();
    let before = snapshot(&fixture.root);
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(other_lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyLeaseInvalid);
    assert!(!failure.effect_started());
    let (request, permit, _) = failure.into_pre_effect().unwrap().into_parts();
    assert_eq!(snapshot(&fixture.root), before);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        permit,
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
}
