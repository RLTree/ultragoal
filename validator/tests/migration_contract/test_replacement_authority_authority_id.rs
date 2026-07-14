impl ReplacementEvidenceAuthority for TestReplacementAuthority {
fn authority_id(&self) -> &str {
        &self.authority_id
    }

    fn reviewer_id(&self, route_id: &str) -> Option<&str> {
        self.route_matches(route_id)
            .then_some(self.reviewer_id.as_str())
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }

    fn nonce_sha256(&self, route_id: &str) -> Option<&str> {
        self.route_matches(route_id)
            .then_some(self.nonce_sha256.as_str())
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

    fn current_binding(&self) -> (&str, &str, &str, &str, &str) {
        (
            &self.context_id,
            &self.candidate_id,
            &self.catalog_id,
            &self.read_session_id,
            &self.inventory_sha256,
        )
    }

    fn old_behavior(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)> {
        self.route_matches(route_id).then_some((
            self.old_behavior_id.as_str(),
            self.old_verdict,
            self.old_result_sha256.as_str(),
        ))
    }

    fn new_behavior(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)> {
        self.route_matches(route_id).then_some((
            self.new_behavior_id.as_str(),
            self.new_verdict,
            self.new_result_sha256.as_str(),
        ))
    }

    fn representative_journey(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)> {
        self.route_matches(route_id).then_some((
            self.journey_execution_id.as_str(),
            self.journey_verdict,
            self.journey_result_sha256.as_str(),
        ))
    }

    fn false_pass_control_results(
        &self,
        route_id: &str,
    ) -> Option<&BTreeMap<String, (EvidenceVerdict, String)>> {
        self.route_matches(route_id).then_some(&self.controls)
    }

    fn rollback_execution(&self, route_id: &str) -> Option<(&str, EvidenceVerdict, &str)> {
        self.route_matches(route_id).then_some((
            self.rollback_execution_id.as_str(),
            self.rollback_verdict,
            self.rollback_result_sha256.as_str(),
        ))
    }

    fn issue_attestation(
        &mut self,
        route_id: &str,
        binding_sha256: &str,
    ) -> Result<String, MigrationError> {
        if !self.route_matches(route_id) {
            return Err(MigrationError::new(
                "migration-replacement-evidence-issuance-refused",
            ));
        }
        Ok(self.attestation(route_id, binding_sha256))
    }

    fn verify_attestation(
        &self,
        route_id: &str,
        binding_sha256: &str,
        attestation_sha256: &str,
    ) -> bool {
        self.route_matches(route_id)
            && self.attestation(route_id, binding_sha256) == attestation_sha256
    }

    fn consume_once(
        &mut self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
    ) -> Result<String, MigrationError> {
        let expected_attestation = self.attestation(route_id, binding_sha256);
        let expected_evidence_id = test_digest(
            format!(
                "replacement-evidence|{}|{}",
                binding_sha256, expected_attestation
            )
            .as_bytes(),
        );
        if !self.route_matches(route_id)
            || expected_attestation != attestation_sha256
            || expected_evidence_id != evidence_id
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-replayed",
            ));
        }
        let receipt = self.consumption(route_id, binding_sha256, evidence_id, attestation_sha256);
        if self
            .ledger
            .lock()
            .unwrap()
            .consumed
            .insert(evidence_id.to_owned(), receipt.clone())
            .is_some()
        {
            return Err(MigrationError::new(
                "migration-replacement-evidence-replayed",
            ));
        }
        if self.mutate_after_consume {
            self.candidate_id = sha('0');
        }
        Ok(receipt)
    }

    fn verify_consumed(
        &self,
        route_id: &str,
        binding_sha256: &str,
        evidence_id: &str,
        attestation_sha256: &str,
        consumption_sha256: &str,
    ) -> bool {
        self.ledger
            .lock()
            .unwrap()
            .consumed
            .get(evidence_id)
            .is_some_and(|receipt| receipt == consumption_sha256)
            && self.consumption(route_id, binding_sha256, evidence_id, attestation_sha256)
                == consumption_sha256
    }

    fn bind_plan_target(
        &mut self,
        binding: &ReplacementLedgerBinding,
    ) -> Result<(), MigrationError> {
        let mut ledger = self.ledger.lock().unwrap();
        if !self.route_matches(binding.route_id())
            || self.authority_id != binding.authority_id()
            || self.session_id != binding.authority_session_id()
            || ledger.revoked.contains(binding.evidence_id())
            || ledger
                .consumed
                .get(binding.evidence_id())
                .is_none_or(|receipt| receipt != binding.consumption_sha256())
        {
            return Err(MigrationError::new(
                "migration-replacement-ledger-binding-refused",
            ));
        }
        match ledger.plan_bindings.get(binding.evidence_id()) {
            Some(existing) if existing != binding.ledger_binding_sha256() => Err(
                MigrationError::new("migration-replacement-ledger-binding-conflict"),
            ),
            Some(_) => Ok(()),
            None => {
                ledger.plan_bindings.insert(
                    binding.evidence_id().to_owned(),
                    binding.ledger_binding_sha256().to_owned(),
                );
                Ok(())
            }
        }
    }

    fn verify_plan_target_consumed(&mut self, binding: &ReplacementLedgerBinding) -> bool {
        let ledger = self.ledger.lock().unwrap();
        self.ledger_row_is_current(binding, &ledger)
    }

    fn claim_final_reconciliation(
        &mut self,
        binding: &ReplacementLedgerBinding,
    ) -> Result<String, MigrationError> {
        let claim = self.final_claim(binding);
        let mut ledger = self.ledger.lock().unwrap();
        if !self.ledger_row_is_current(binding, &ledger)
            || ledger.final_claims.contains_key(binding.evidence_id())
        {
            return Err(MigrationError::new(
                "migration-replacement-ledger-final-replay",
            ));
        }
        ledger
            .final_claims
            .insert(binding.evidence_id().to_owned(), claim.clone());
        if self.rollback_after_final_claim {
            ledger.consumed.remove(binding.evidence_id());
        }
        if self.revoke_after_final_claim {
            ledger.revoked.insert(binding.evidence_id().to_owned());
        }
        Ok(claim)
    }

    fn verify_final_claim(
        &mut self,
        binding: &ReplacementLedgerBinding,
        claim_sha256: &str,
    ) -> bool {
        self.final_verify_calls += 1;
        let mut ledger = self.ledger.lock().unwrap();
        if self.revoke_on_final_verify_call == Some(self.final_verify_calls) {
            ledger.revoked.insert(binding.evidence_id().to_owned());
        }
        self.ledger_row_is_current(binding, &ledger)
            && ledger
                .final_claims
                .get(binding.evidence_id())
                .is_some_and(|claim| claim == claim_sha256)
            && self.final_claim(binding) == claim_sha256
    }
}
