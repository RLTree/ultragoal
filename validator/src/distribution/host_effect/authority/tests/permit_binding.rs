fn repeated_digest(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn binding() -> HostEffectPermitBinding {
    HostEffectPermitBinding {
        context_id: repeated_digest('1'),
        candidate_id: repeated_digest('2'),
        package_identity_sha256: repeated_digest('3'),
        journey_binding_sha256: repeated_digest('4'),
        session_issuance_sha256: repeated_digest('5'),
        lifecycle_plan_sha256: repeated_digest('6'),
        lifecycle_intent: "install".to_owned(),
        expected_pre_state_sha256: repeated_digest('7'),
        expected_post_state_sha256: repeated_digest('8'),
        rollback_policy_sha256: repeated_digest('9'),
        reconciliation_policy_sha256: repeated_digest('a'),
        host_scope_sha256: repeated_digest('b'),
        host_capability_sha256: repeated_digest('c'),
        required_capabilities_sha256: repeated_digest('d'),
        external_request_sha256: repeated_digest('e'),
        command_plan_sha256: repeated_digest('f'),
        argv_sha256: repeated_digest('1'),
        executable_identity_sha256: repeated_digest('2'),
        target_identity_sha256: repeated_digest('3'),
        target_generation: 1,
        issued_at_unix_ms: 1_000,
        expires_at_unix_ms: 2_000,
        expected_head_sha256: repeated_digest('4'),
        lifecycle_record: None,
        lifecycle_record_sha256: None,
        decision: HostEffectDecision::Authorize,
    }
}

fn issue_error(
    result: Result<(HostEffectPermit, HostEffectReservation), HostEffectAuthorityError>,
) -> HostEffectAuthorityErrorId {
    match result {
        Ok(_) => panic!("permit unexpectedly issued"),
        Err(error) => error.id(),
    }
}

#[test]
fn issued_permit_verifies_only_under_exact_authority_and_binding() {
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let (permit, reservation) = authority.issue(binding()).unwrap();
    authority.verify(&permit, 1_500).unwrap();
    assert_eq!(reservation.permit_id(), permit.permit_id());
    assert_eq!(
        reservation.semantic_key_sha256(),
        permit.semantic_key_sha256()
    );
}

#[test]
fn field_mutation_wrong_authority_and_time_fail_closed() {
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.binding.candidate_id = repeated_digest('a');
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidBinding
    );

    let (permit, _) = authority.issue(binding()).unwrap();
    assert_eq!(
        authority.verify(&permit, 999).unwrap_err().id(),
        HostEffectAuthorityErrorId::NotYetValid
    );
    assert_eq!(
        authority.verify(&permit, 2_001).unwrap_err().id(),
        HostEffectAuthorityErrorId::Expired
    );
    let other =
        HostEffectAuthority::generate("other-root".to_owned(), "host-ledger".to_owned()).unwrap();
    assert_eq!(
        other.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::WrongAuthority
    );
}

#[test]
fn nonce_and_permit_are_unique_while_semantic_key_is_stable() {
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let (left, _) = authority.issue(binding()).unwrap();
    let (right, _) = authority.issue(binding()).unwrap();
    assert_ne!(left.nonce_sha256(), right.nonce_sha256());
    assert_ne!(left.permit_id(), right.permit_id());
    assert_eq!(left.semantic_key_sha256(), right.semantic_key_sha256());

    let mut reissued = binding();
    reissued.session_issuance_sha256 = repeated_digest('9');
    reissued.external_request_sha256 = repeated_digest('8');
    reissued.executable_identity_sha256 = repeated_digest('7');
    reissued.target_identity_sha256 = repeated_digest('6');
    reissued.target_generation = 2;
    reissued.issued_at_unix_ms = 2_000;
    reissued.expires_at_unix_ms = 3_000;
    let (reissued, _) = authority.issue(reissued).unwrap();
    assert_eq!(left.semantic_key_sha256(), reissued.semantic_key_sha256());
}

#[test]
fn refusal_and_overlong_ttl_never_issue() {
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let mut refused = binding();
    refused.decision = HostEffectDecision::Refuse;
    assert_eq!(
        issue_error(authority.issue(refused)),
        HostEffectAuthorityErrorId::Refused
    );
    let mut stale = binding();
    stale.expires_at_unix_ms = stale.issued_at_unix_ms + MAX_PERMIT_TTL_MS + 1;
    assert_eq!(
        issue_error(authority.issue(stale)),
        HostEffectAuthorityErrorId::InvalidBinding
    );
}
