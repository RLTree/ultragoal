#[test]
fn every_binding_and_permit_field_mutation_fails_closed() {
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let mutations: &[fn(&mut HostEffectPermitBinding)] = &[
        |row| row.context_id = repeated_digest('a'),
        |row| row.candidate_id = repeated_digest('a'),
        |row| row.package_identity_sha256 = repeated_digest('a'),
        |row| row.journey_binding_sha256 = repeated_digest('a'),
        |row| row.session_issuance_sha256 = repeated_digest('a'),
        |row| row.lifecycle_plan_sha256 = repeated_digest('a'),
        |row| row.lifecycle_intent = "remove".to_owned(),
        |row| row.expected_pre_state_sha256 = repeated_digest('a'),
        |row| row.expected_post_state_sha256 = repeated_digest('a'),
        |row| row.rollback_policy_sha256 = repeated_digest('b'),
        |row| row.reconciliation_policy_sha256 = repeated_digest('b'),
        |row| row.host_scope_sha256 = repeated_digest('a'),
        |row| row.host_capability_sha256 = repeated_digest('a'),
        |row| row.required_capabilities_sha256 = repeated_digest('a'),
        |row| row.external_request_sha256 = repeated_digest('a'),
        |row| row.command_plan_sha256 = repeated_digest('a'),
        |row| row.argv_sha256 = repeated_digest('a'),
        |row| row.executable_identity_sha256 = repeated_digest('a'),
        |row| row.target_identity_sha256 = repeated_digest('a'),
        |row| row.target_generation += 1,
        |row| row.issued_at_unix_ms += 1,
        |row| row.expires_at_unix_ms += 1,
        |row| row.expected_head_sha256 = repeated_digest('a'),
        |row| row.decision = HostEffectDecision::Refuse,
    ];
    for mutate in mutations {
        let (mut permit, _) = authority.issue(binding()).unwrap();
        mutate(&mut permit.binding);
        assert!(authority.verify(&permit, 1_500).is_err());
    }

    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.nonce[0] ^= 1;
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidBinding
    );
    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.nonce_sha256 = repeated_digest('a');
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidBinding
    );
    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.binding_sha256 = repeated_digest('a');
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidBinding
    );
    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.semantic_key_sha256 = repeated_digest('a');
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidBinding
    );
    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.tag[0] ^= 1;
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidMac
    );
    let (mut permit, _) = authority.issue(binding()).unwrap();
    permit.permit_id = repeated_digest('a');
    assert_eq!(
        authority.verify(&permit, 1_500).unwrap_err().id(),
        HostEffectAuthorityErrorId::InvalidBinding
    );
    let identity_mutations: &[fn(&mut HostEffectPermit)] = &[
        |row: &mut HostEffectPermit| row.issuer_id = "other-root".to_owned(),
        |row: &mut HostEffectPermit| row.ledger_id = "other-ledger".to_owned(),
        |row: &mut HostEffectPermit| row.key_id = repeated_digest('a'),
    ];
    for mutate_identity in identity_mutations {
        let (mut permit, _) = authority.issue(binding()).unwrap();
        mutate_identity(&mut permit);
        assert_eq!(
            authority.verify(&permit, 1_500).unwrap_err().id(),
            HostEffectAuthorityErrorId::WrongAuthority
        );
    }
}

#[test]
fn semantic_key_is_retry_stable_but_changes_for_distinct_effects() {
    let authority =
        HostEffectAuthority::generate("root-actor".to_owned(), "host-ledger".to_owned()).unwrap();
    let (baseline, _) = authority.issue(binding()).unwrap();
    let baseline = baseline.semantic_key_sha256().to_owned();

    let retry_mutations: &[fn(&mut HostEffectPermitBinding)] = &[
        |row| row.session_issuance_sha256 = repeated_digest('a'),
        |row| row.external_request_sha256 = repeated_digest('a'),
        |row| row.executable_identity_sha256 = repeated_digest('a'),
        |row| row.target_identity_sha256 = repeated_digest('a'),
        |row| row.target_generation += 1,
        |row| row.issued_at_unix_ms += 1,
        |row| row.expires_at_unix_ms += 1,
        |row| row.expected_head_sha256 = repeated_digest('a'),
    ];
    for mutate in retry_mutations {
        let mut row = binding();
        mutate(&mut row);
        let (permit, _) = authority.issue(row).unwrap();
        assert_eq!(permit.semantic_key_sha256(), baseline.as_str());
    }

    let effect_mutations: &[fn(&mut HostEffectPermitBinding)] = &[
        |row| row.context_id = repeated_digest('a'),
        |row| row.candidate_id = repeated_digest('a'),
        |row| row.package_identity_sha256 = repeated_digest('a'),
        |row| row.journey_binding_sha256 = repeated_digest('a'),
        |row| row.lifecycle_plan_sha256 = repeated_digest('a'),
        |row| row.lifecycle_intent = "remove".to_owned(),
        |row| row.host_scope_sha256 = repeated_digest('a'),
        |row| row.command_plan_sha256 = repeated_digest('a'),
    ];
    for mutate in effect_mutations {
        let mut row = binding();
        mutate(&mut row);
        let (permit, _) = authority.issue(row).unwrap();
        assert_ne!(permit.semantic_key_sha256(), baseline.as_str());
    }
}
