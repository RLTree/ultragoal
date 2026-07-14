use super::*;

#[test]
pub(crate) fn authority_rejects_short_secrets_nonces_and_invalid_validity_windows() {
    let failure = match TestRepositoryFitPermitAuthority::new(b"short") {
        Err(failure) => failure,
        Ok(_) => panic!("short root secret unexpectedly created an authority"),
    };
    assert_eq!(failure.id(), AdapterErrorId::ApplyPermitInvalid);

    let fixture = Fixture::new("invalid-authority-inputs");
    let context = fixture.context();
    let request = fixture.request(&context);
    let authority = new_authority();
    let inverted_nonce = nonce("inverted-window");
    let overlong_nonce = nonce("overlong-window");
    for (issued, expires, supplied_nonce) in [
        (10, 20, b"short".as_slice()),
        (20, 19, inverted_nonce.as_slice()),
        (20, 321, overlong_nonce.as_slice()),
    ] {
        let failure = match authority.issue(
            &context,
            &request,
            fixture.effects(&request),
            issued,
            expires,
            supplied_nonce,
        ) {
            Err(failure) => failure,
            Ok(_) => panic!("invalid authority bounds unexpectedly issued a permit"),
        };
        assert_eq!(failure.id(), AdapterErrorId::ApplyPermitInvalid);
    }
    let (permit, lease) = authority
        .issue(
            &context,
            &request,
            fixture.effects(&request),
            20,
            30,
            &nonce("valid-after-invalid-inputs"),
        )
        .unwrap();
    let outcome = expect_apply_ok(apply_with_root_permit(
        &context,
        request,
        Some(permit),
        Some(lease),
        20,
    ));
    assert_eq!(outcome.status(), "applied");
}
