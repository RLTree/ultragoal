fn evidence(
    inventory: &MigrationInventory,
    route: &CompatibilityRoute,
    authority: &mut TestReplacementAuthority,
) -> ReplacementEvidence {
    ReplacementEvidence::issue(inventory, route, authority).unwrap()
}

fn plan_with_authority(
    inventory: &MigrationInventory,
) -> (MigrationPlan, TestReplacementAuthority) {
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(inventory, &route);
    let evidence = evidence(inventory, &route, &mut authority);
    let plan =
        MigrationPlan::build(inventory, vec![route], vec![evidence], &mut authority).unwrap();
    (plan, authority)
}

struct TestReviewAuthority {
    authority_id: String,
    reviewer_id: String,
    session_id: String,
    nonce_sha256: String,
    issued_at: u64,
    expires_at: u64,
    now: u64,
    context_id: String,
    candidate_id: String,
    catalog_id: String,
    inventory_sha256: String,
    decision: Od009Decision,
    secret: String,
    consumed: BTreeSet<String>,
    mutate_after_consume: bool,
}

impl TestReviewAuthority {
    fn current(inventory: &MigrationInventory, decision: Od009Decision) -> Self {
        Self {
            authority_id: "root-retirement-review-authority".to_owned(),
            reviewer_id: "retirement-reviewer".to_owned(),
            session_id: sha('8'),
            nonce_sha256: sha('a'),
            issued_at: 1_000,
            expires_at: 2_000,
            now: 1_500,
            context_id: sha('c'),
            candidate_id: sha('d'),
            catalog_id: sha('e'),
            inventory_sha256: inventory.inventory_sha256().to_owned(),
            decision,
            secret: "test-only-retirement-review-secret".to_owned(),
            consumed: BTreeSet::new(),
            mutate_after_consume: false,
        }
    }

    fn attestation(&self, binding_sha256: &str) -> String {
        test_digest(
            format!(
                "{}|{}|{}|{}",
                self.secret, self.authority_id, self.reviewer_id, binding_sha256
            )
            .as_bytes(),
        )
    }
}

impl RetirementReviewAuthority for TestReviewAuthority {
    fn authority_id(&self) -> &str {
        &self.authority_id
    }

    fn reviewer_id(&self) -> &str {
        &self.reviewer_id
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

    fn od009_decision(&self) -> Od009Decision {
        self.decision
    }

    fn issue_attestation(&mut self, binding_sha256: &str) -> Result<String, MigrationError> {
        Ok(self.attestation(binding_sha256))
    }

    fn verify_and_consume(
        &mut self,
        binding_sha256: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> bool {
        let expected_attestation = self.attestation(binding_sha256);
        let expected_review_id = test_digest(
            format!("retirement-review|{binding_sha256}|{expected_attestation}").as_bytes(),
        );
        let accepted = expected_attestation == attestation_sha256
            && expected_review_id == review_id
            && self.consumed.insert(review_id.to_owned());
        if accepted && self.mutate_after_consume {
            self.candidate_id = sha('0');
        }
        accepted
    }
}
