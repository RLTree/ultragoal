#[test]
fn destructive_authorization_issuance_rejects_shared_authority_and_ambiguous_scope() {
    let inventory = clean_inventory();
    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    let (review, review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    for case in 0..7 {
        let mut authority = TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
        match case {
            0 => authority.authority_id = "root-retirement-review-authority".to_owned(),
            1 => authority.principal_id = "retirement-reviewer".to_owned(),
            2 => authority.session_id = sha('8'),
            3 => authority.nonce_sha256 = sha('a'),
            4 => authority.now = authority.expires_at + 1,
            5 => authority.effect_scope_sha256 = sha('1'),
            6 => authority.inventory_sha256 = sha('2'),
            _ => unreachable!(),
        }
        assert_eq!(
            DestructiveAuthorization::issue(
                &plan,
                "retire-LEGACY-SKILL:old",
                &inventory,
                &review,
                &review_authority,
                &mut authority,
            )
            .unwrap_err()
            .code(),
            "migration-destructive-authorization-issuance-refused"
        );
    }
}
