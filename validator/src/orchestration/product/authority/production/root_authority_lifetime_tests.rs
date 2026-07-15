fn lifetime_digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn lifetime_test_authority() -> RootAuthority {
    RootAuthority::from_secret(
        Actor::parse("lifetime-root").unwrap(),
        b"deep-lifetime-boundary-secret-0123456789",
    )
    .unwrap()
}

#[test]
fn deepest_issuer_refuses_u64_max_lifetime() {
    let authority = lifetime_test_authority();
    let workspace_identity = lifetime_digest('c');
    let journal_head_identity = lifetime_digest('d');
    let error = authority
        .issue(RootPermitIssuance {
            operation: RootOperation::Resume,
            binding: Binding::new(&lifetime_digest('a'), &lifetime_digest('b')).unwrap(),
            workspace_identity: &workspace_identity,
            journal_head_identity: &journal_head_identity,
            issued_tick: 1,
            expires_tick: u64::MAX,
            nonce: b"deep-lifetime-issuer-nonce-012345",
            target: PermitTarget::default(),
            decision_binding: PermitDecisionBinding::ActionOnly,
        })
        .unwrap_err();
    assert_eq!(error, ProductError::AuthorityInvalid);
}

#[test]
fn deepest_verifier_refuses_authentic_u64_max_lifetime() {
    let authority = lifetime_test_authority();
    let binding = Binding::new(&lifetime_digest('a'), &lifetime_digest('b')).unwrap();
    let workspace_identity = lifetime_digest('c');
    let journal_head_identity = lifetime_digest('d');
    let target = PermitTarget::default();
    let mut permit = RootPermit {
        schema_version: AUTHORITY_SCHEMA.to_owned(),
        root_actor: authority.root_actor.as_str().to_owned(),
        operation: RootOperation::Resume,
        binding: binding.clone(),
        workspace_identity: workspace_identity.clone(),
        journal_head_identity: journal_head_identity.clone(),
        issued_tick: 1,
        expires_tick: u64::MAX,
        nonce_digest: digest(b"deep-lifetime-verifier-nonce-012345"),
        target: target.clone(),
        decision_binding: PermitDecisionBinding::ActionOnly,
        authenticator: String::new(),
    };
    permit.authenticator = authority.authenticate(&permit).unwrap();
    let error = authority
        .verify_action(RootActionPermitVerification {
            permit: &permit,
            expected_root: &authority.root_actor,
            operation: RootOperation::Resume,
            binding: &binding,
            workspace_identity: &workspace_identity,
            journal_head_identity: &journal_head_identity,
            tick: 1,
            target: &target,
        })
        .unwrap_err();
    assert_eq!(error, ProductError::AuthorityInvalid);
}
