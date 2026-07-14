struct TestEffectAuthority {
    authority_id: String,
    principal_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at: u64,
    expires_at: u64,
    now: u64,
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    inventory_sha256: String,
    effect_scope_sha256: String,
    secret: String,
    consumed: BTreeSet<String>,
    mutate_after_consume: bool,
}

impl TestEffectAuthority {
    fn current(inventory: &MigrationInventory, effect_scope_sha256: &str) -> Self {
        Self {
            authority_id: "root-destructive-effect-authority".to_owned(),
            principal_id: "root-destructive-effect-principal".to_owned(),
            session_id: sha('9'),
            nonce_sha256: sha('0'),
            issued_at: 1_100,
            expires_at: 2_100,
            now: 1_500,
            context_id: sha('c'),
            candidate_id: sha('d'),
            catalog_id: sha('e'),
            inventory_sha256: inventory.inventory_sha256().to_owned(),
            effect_scope_sha256: effect_scope_sha256.to_owned(),
            secret: "test-only-destructive-effect-secret".to_owned(),
            consumed: BTreeSet::new(),
            mutate_after_consume: false,
        }
    }

    fn attestation(&self, binding_sha256: &str) -> String {
        test_digest(
            format!(
                "{}|{}|{}|{}",
                self.secret, self.authority_id, self.principal_id, binding_sha256
            )
            .as_bytes(),
        )
    }
}

impl DestructiveEffectAuthority for TestEffectAuthority {
    fn authority_id(&self) -> &str {
        &self.authority_id
    }

    fn principal_id(&self) -> &str {
        &self.principal_id
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn nonce_sha256(&self) -> &str {
        &self.nonce_sha256
    }

    fn issued_at_unix_ms(&self) -> u64 {
        self.issued_at
    }

    fn expires_at_unix_ms(&self) -> u64 {
        self.expires_at
    }

    fn now_unix_ms(&self) -> u64 {
        self.now
    }

    fn current_binding(&self) -> (&str, &str, &str, &str) {
        (
            &self.context_id,
            &self.candidate_id,
            &self.catalog_id,
            &self.inventory_sha256,
        )
    }

    fn effect_scope_sha256(&self) -> &str {
        &self.effect_scope_sha256
    }

    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, MigrationError> {
        Ok(self.attestation(binding_sha256))
    }

    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        authorization_id: &str,
        attestation_sha256: &str,
    ) -> bool {
        let expected_attestation = self.attestation(binding_sha256);
        let expected_authorization_id = test_digest(
            format!("destructive-authorization|{binding_sha256}|{expected_attestation}").as_bytes(),
        );
        let accepted = expected_attestation == attestation_sha256
            && expected_authorization_id == authorization_id
            && self.consumed.insert(authorization_id.to_owned());
        if accepted && self.mutate_after_consume {
            self.effect_scope_sha256 = sha('0');
        }
        accepted
    }
}

fn issue_review(
    plan: &MigrationPlan,
    inventory: &MigrationInventory,
    decision: Od009Decision,
    reviewer_id: &str,
) -> (RetirementReview, TestReviewAuthority) {
    let mut authority = TestReviewAuthority::current(inventory, decision);
    authority.reviewer_id = reviewer_id.to_owned();
    let review =
        RetirementReview::issue(plan, "retire-LEGACY-SKILL:old", inventory, &mut authority)
            .unwrap();
    (review, authority)
}

fn preservation_decision(
    plan: &MigrationPlan,
    target_id: &str,
    inventory: &MigrationInventory,
    replacement_authority: &mut TestReplacementAuthority,
    reviewer_id: &str,
) -> RetirementDecision {
    let mut authority =
        TestReviewAuthority::current(inventory, Od009Decision::PreservePhysicalArtifact);
    authority.reviewer_id = reviewer_id.to_owned();
    let review = RetirementReview::issue(plan, target_id, inventory, &mut authority).unwrap();
    RetirementDecision::reconcile_preservation(
        plan,
        target_id,
        inventory,
        &review,
        replacement_authority,
        &mut authority,
    )
}

fn assert_review_issue_refused(
    plan: &MigrationPlan,
    inventory: &MigrationInventory,
    mut authority: TestReviewAuthority,
) {
    assert_eq!(
        RetirementReview::issue(plan, "retire-LEGACY-SKILL:old", inventory, &mut authority,)
            .unwrap_err()
            .code(),
        "migration-retirement-review-issuance-refused"
    );
}

#[test]
fn current_inventory_builds_a_deterministic_read_only_plan() {
    let inventory = clean_inventory();
    let (first, _first_authority) = plan_with_authority(&inventory);
    let (second, _second_authority) = plan_with_authority(&inventory);
    assert_eq!(first.plan_sha256(), second.plan_sha256());
    assert_eq!(first.targets().len(), 1);
    assert_eq!(first.targets()[0].target_id(), "retire-LEGACY-SKILL:old");
    assert!(first.verify_current(&inventory).is_ok());
}

#[test]
fn unknown_route_source_is_rejected() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:unknown", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut authority);
    let error =
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut authority).unwrap_err();
    assert_eq!(error.code(), "migration-route-source-unknown");
}
