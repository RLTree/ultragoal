use super::*;

impl RepositoryFitPermitEffects for UndeclaredWrite {
    fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        self.inner.read_unix_mode(path)
    }
}

#[test]
pub(crate) fn green_receipt_without_effect_and_undeclared_write_cannot_fabricate_success() {
    let fixture = Fixture::new("green-without-effect");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            NoEffect {
                inner: fixture.effects(&request),
            },
            10,
            20,
            &nonce("green-without-effect"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyRolledBack);
    assert!(failure.rollback_complete());

    let fixture = Fixture::new("undeclared-write");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            UndeclaredWrite {
                inner: fixture.effects(&request),
                root: fixture.root.clone(),
                fired: false,
            },
            10,
            20,
            &nonce("undeclared-write"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(
        !failure
            .error()
            .to_string()
            .contains("do-not-echo-this-private-canary")
    );
}

#[test]
pub(crate) fn root_replacement_after_the_first_effect_is_terminally_ambiguous() {
    let fixture = Fixture::new("root-swap-after-effect");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            RootSwapAfterFirstEffect {
                inner: fixture.effects(&request),
                root: fixture.root.clone(),
                displaced: fixture.container.join("displaced-after-effect"),
                fired: false,
            },
            10,
            20,
            &nonce("root-swap-after-effect"),
        )
        .unwrap();
    let failure = expect_apply_failure(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(failure.error().id(), AdapterErrorId::ApplyOutcomeAmbiguous);
    assert!(failure.effect_started());
    assert!(!failure.rollback_complete());
}

#[test]
pub(crate) fn verify_cannot_consume_or_substitute_for_apply_and_secrets_never_echo() {
    let fixture = Fixture::new("verify-as-apply");
    let context = fixture.context();
    let request = fixture.request(&context);
    let canary_secret = b"root-secret-canary-that-must-never-echo-0001";
    let canary_nonce = b"nonce-canary-that-must-never-echo-0000000001";
    let authority = TestRepositoryFitPermitAuthority::new(canary_secret).unwrap();
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            canary_nonce,
        )
        .unwrap();
    let debug = format!("{permit:?}");
    assert!(!debug.contains(std::str::from_utf8(canary_secret).unwrap()));
    assert!(!debug.contains(std::str::from_utf8(canary_nonce).unwrap()));
    let before = snapshot(&fixture.root);
    let verification = verify_target(&context).unwrap();
    assert!(!verification.idempotent());
    assert_eq!(snapshot(&fixture.root), before);
    assert_eq!(permit_seal_stage_for_test(&request), 0);
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        10,
    ));
    assert_eq!(outcome.status(), "applied");
}

#[test]
pub(crate) fn authority_rejects_nonce_reuse_and_duplicate_request_issuance() {
    let fixture = Fixture::new("issuance-replay");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let (_permit, _lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            10,
            20,
            &nonce("issuance-replay"),
        )
        .unwrap();
    let failure = match authority.issue(
        &context,
        &request,
        fixture.effects(&request),
        10,
        20,
        &nonce("issuance-replay-second-nonce"),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("duplicate request issuance unexpectedly succeeded"),
    };
    assert_eq!(failure.id(), AdapterErrorId::ApplyPermitReplayed);
}
