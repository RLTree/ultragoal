impl TestReplacementAuthority {
    fn current(inventory: &MigrationInventory, route: &CompatibilityRoute) -> Self {
        let controls = [
            "proof-artifact",
            "receipt-production",
            "score-only",
            "test-manipulation",
            "verbosity",
        ]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                (
                    EvidenceVerdict::CausalFailure,
                    test_digest(format!("control-result:{name}").as_bytes()),
                ),
            )
        })
        .collect();
        Self {
            authority_id: "root-observed-replacement-authority".to_owned(),
            reviewer_id: "replacement-reviewer".to_owned(),
            session_id: test_digest(b"replacement-authority-session"),
            nonce_sha256: test_digest(format!("replacement-nonce:{}", route.route_id()).as_bytes()),
            issued_at: 1_000,
            expires_at: 2_000,
            now: 1_500,
            context_id: inventory.live_context_id().to_owned(),
            candidate_id: inventory.candidate_id().to_owned(),
            catalog_id: inventory.catalog_id().to_owned(),
            read_session_id: inventory.read_session_id().to_owned(),
            inventory_sha256: inventory.inventory_sha256().to_owned(),
            route_id: route.route_id().to_owned(),
            old_behavior_id: route.source_id().to_owned(),
            old_verdict: EvidenceVerdict::Passed,
            old_result_sha256: test_digest(b"observed-old-behavior-result"),
            new_behavior_id: route.canonical_target_id().to_owned(),
            new_verdict: EvidenceVerdict::Passed,
            new_result_sha256: test_digest(b"observed-new-behavior-result"),
            journey_execution_id: "representative-migration-journey".to_owned(),
            journey_verdict: EvidenceVerdict::Passed,
            journey_result_sha256: test_digest(b"representative-journey-result"),
            controls,
            rollback_execution_id: "observed-rollback-execution".to_owned(),
            rollback_verdict: EvidenceVerdict::Passed,
            rollback_result_sha256: test_digest(b"observed-rollback-result"),
            secret: "test-only-replacement-authority-secret".to_owned(),
            ledger: Arc::new(Mutex::new(TestReplacementLedger::default())),
            mutate_after_consume: false,
            rollback_after_final_claim: false,
            revoke_after_final_claim: false,
            revoke_on_final_verify_call: None,
            final_verify_calls: 0,
        }
    }

    fn route_matches(&self, route_id: &str) -> bool {
        self.route_id == route_id
    }

    fn attestation(&self, route_id: &str, binding_sha256: &str) -> String {
        test_digest(
            format!(
                "{}|attest|{}|{}|{}|{}",
                self.secret, route_id, self.authority_id, self.reviewer_id, binding_sha256
            )
            .as_bytes(),
        )
    }

    fn consumption(
        &self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
    ) -> String {
        test_digest(
            format!(
                "{}|consume|{}|{}|{}|{}",
                self.secret, route_id, binding_sha256, evidence_id, attestation_sha256
            )
            .as_bytes(),
        )
    }

    fn final_claim(&self, binding: &ReplacementLedgerBinding) -> String {
        test_digest(
            format!(
                "{}|final-reconciliation|{}|{}",
                self.secret,
                binding.evidence_id(),
                binding.ledger_binding_sha256(),
            )
            .as_bytes(),
        )
    }

    fn ledger_row_is_current(
        &self,
        binding: &ReplacementLedgerBinding,
        ledger: &TestReplacementLedger,
    ) -> bool {
        self.route_matches(binding.route_id())
            && self.authority_id == binding.authority_id()
            && self.session_id == binding.authority_session_id()
            && ledger
                .consumed
                .get(binding.evidence_id())
                .is_some_and(|receipt| receipt == binding.consumption_sha256())
            && ledger
                .plan_bindings
                .get(binding.evidence_id())
                .is_some_and(|digest| digest == binding.ledger_binding_sha256())
            && !ledger.revoked.contains(binding.evidence_id())
    }

    fn rollback_consumption(&mut self) {
        self.ledger.lock().unwrap().consumed.clear();
    }

    fn revoke(&mut self) {
        let evidence_ids = self
            .ledger
            .lock()
            .unwrap()
            .consumed
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        self.ledger.lock().unwrap().revoked.extend(evidence_ids);
    }

    fn detached_without_ledger(&self) -> Self {
        let mut detached = self.clone();
        detached.ledger = Arc::new(Mutex::new(TestReplacementLedger::default()));
        detached
    }
}
