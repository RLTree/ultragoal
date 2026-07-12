#![allow(dead_code)]

#[path = "../src/migration/mod.rs"]
mod migration;

use migration::{
    CompatibilityRoute, DestructiveAuthorization, DestructiveEffectAuthority, EvidenceVerdict,
    InventorySurface, MigrationError, MigrationInventory, MigrationPlan, MigrationPlanProjection,
    Od009Decision, ReplacementEvidence, ReplacementEvidenceAuthority, ReplacementLedgerBinding,
    RetirementDecision, RetirementReview, RetirementReviewAuthority, RetirementStatus,
    RetirementTarget, RetirementTargetProjection, SurfaceFileKind, SurfaceStatus,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn require_type<T>() {}

#[test]
fn required_migration_api_types_compile_in_the_leased_module() {
    require_type::<MigrationPlan>();
    require_type::<MigrationPlanProjection>();
    require_type::<CompatibilityRoute>();
    require_type::<RetirementTarget>();
    require_type::<RetirementTargetProjection>();
    require_type::<RetirementDecision>();
}

fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

fn surface(
    id: &str,
    status: SurfaceStatus,
    readers: Vec<String>,
    writers: Vec<String>,
    routes: Vec<String>,
    generated: Vec<String>,
) -> InventorySurface {
    InventorySurface::observed(
        id,
        "skill",
        format!("skills/{}.md", id.replace(':', "-")),
        if id.starts_with("LEGACY") {
            sha('a')
        } else {
            sha('b')
        },
        SurfaceFileKind::Regular,
        1,
        status,
        readers,
        writers,
        routes,
        generated,
    )
}

fn inventory_with_source(source: InventorySurface, session: char) -> MigrationInventory {
    MigrationInventory::new(
        sha('c'),
        sha('d'),
        sha('e'),
        sha(session),
        vec![
            source,
            surface(
                "SKILL:current",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
        ],
    )
    .unwrap()
}

fn clean_inventory() -> MigrationInventory {
    inventory_with_source(
        surface(
            "LEGACY-SKILL:old",
            SurfaceStatus::Active,
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        'f',
    )
}

fn route(source: &str, target: &str) -> CompatibilityRoute {
    CompatibilityRoute::new(
        "route-old-to-current",
        source,
        target,
        "migration-owner",
        "Deprecated compatibility route to the canonical successor",
        sha('1'),
        "version-2-boundary",
        "zero-active-references-and-representative-journey",
        sha('2'),
        0,
        true,
    )
}

fn test_digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[derive(Default)]
struct TestReplacementLedger {
    consumed: BTreeMap<String, String>,
    plan_bindings: BTreeMap<String, String>,
    final_claims: BTreeMap<String, String>,
    revoked: BTreeSet<String>,
}

#[derive(Clone)]
struct TestReplacementAuthority {
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
    read_session_id: String,
    inventory_sha256: String,
    route_id: String,
    old_behavior_id: String,
    old_verdict: EvidenceVerdict,
    old_result_sha256: String,
    new_behavior_id: String,
    new_verdict: EvidenceVerdict,
    new_result_sha256: String,
    journey_execution_id: String,
    journey_verdict: EvidenceVerdict,
    journey_result_sha256: String,
    controls: BTreeMap<String, (EvidenceVerdict, String)>,
    rollback_execution_id: String,
    rollback_verdict: EvidenceVerdict,
    rollback_result_sha256: String,
    secret: String,
    ledger: Arc<Mutex<TestReplacementLedger>>,
    mutate_after_consume: bool,
    rollback_after_final_claim: bool,
    revoke_after_final_claim: bool,
    revoke_on_final_verify_call: Option<usize>,
    final_verify_calls: usize,
}

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

#[test]
fn unknown_or_inactive_canonical_target_is_rejected() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:missing");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut authority);
    let unknown =
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut authority).unwrap_err();
    assert_eq!(unknown.code(), "migration-route-target-unknown");
}

#[test]
fn ambiguous_routes_for_one_source_fail_closed() {
    let inventory = MigrationInventory::new(
        sha('c'),
        sha('d'),
        sha('e'),
        sha('f'),
        vec![
            surface(
                "LEGACY-SKILL:old",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            surface(
                "SKILL:current",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
            surface(
                "SKILL:other",
                SurfaceStatus::Active,
                vec![],
                vec![],
                vec![],
                vec![],
            ),
        ],
    )
    .unwrap();
    let second = CompatibilityRoute::new(
        "route-old-to-other",
        "LEGACY-SKILL:old",
        "SKILL:other",
        "migration-owner",
        "Compatibility warning for another target",
        sha('1'),
        "version-2-boundary",
        "zero-active-references",
        sha('2'),
        0,
        true,
    );
    let first = route("LEGACY-SKILL:old", "SKILL:current");
    let mut first_authority = TestReplacementAuthority::current(&inventory, &first);
    let first_evidence = evidence(&inventory, &first, &mut first_authority);
    let mut second_authority = TestReplacementAuthority::current(&inventory, &second);
    let second_evidence = evidence(&inventory, &second, &mut second_authority);
    let error = MigrationPlan::build(
        &inventory,
        vec![first, second],
        vec![first_evidence, second_evidence],
        &mut first_authority,
    )
    .unwrap_err();
    assert!(matches!(
        error.code(),
        "migration-duplicate-replacement-evidence" | "migration-ambiguous-source-route"
    ));
}

#[test]
fn route_requires_bounded_warning_measurement_and_equivalence() {
    let invalid = CompatibilityRoute::new(
        "route",
        "LEGACY-SKILL:old",
        "SKILL:current",
        "owner",
        "silent alias",
        "not-a-digest",
        "boundary",
        "removal",
        "not-a-digest",
        0,
        true,
    );
    assert_eq!(
        invalid.validate().unwrap_err().code(),
        "migration-route-warning-invalid"
    );
}

#[test]
fn stale_replacement_evidence_is_rejected_before_planning() {
    let inventory = clean_inventory();
    let stale_route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &stale_route);
    let mut stale = evidence(&inventory, &stale_route, &mut authority);
    stale.substitute_semantic_for_test(sha('8'));
    assert_eq!(
        MigrationPlan::build(&inventory, vec![stale_route], vec![stale], &mut authority,)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-stale-or-substituted"
    );

    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let mut substituted = evidence(&inventory, &route, &mut authority);
    substituted.substitute_attestation_for_test(sha('9'));
    assert_eq!(
        MigrationPlan::build(&inventory, vec![route], vec![substituted], &mut authority,)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-stale-or-substituted"
    );
}

#[test]
fn replacement_evidence_requires_every_exact_named_false_pass_control() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut missing = TestReplacementAuthority::current(&inventory, &route);
    missing.controls.remove("verbosity");
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut missing)
            .unwrap_err()
            .code(),
        "migration-replacement-controls-incomplete"
    );

    let mut substituted = TestReplacementAuthority::current(&inventory, &route);
    substituted.controls.insert(
        "generic-negative-control".to_owned(),
        (
            EvidenceVerdict::CausalFailure,
            test_digest(b"generic-control-result"),
        ),
    );
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut substituted)
            .unwrap_err()
            .code(),
        "migration-replacement-controls-incomplete"
    );
}

#[test]
fn repeated_hash_replacement_attack_is_rejected_at_observation() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let repeated = sha('a');
    authority.old_result_sha256 = repeated.clone();
    authority.new_result_sha256 = repeated.clone();
    authority.journey_result_sha256 = repeated.clone();
    authority.rollback_result_sha256 = repeated.clone();
    for (_, digest) in authority.controls.values_mut() {
        *digest = repeated.clone();
    }
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut authority)
            .unwrap_err()
            .code(),
        "migration-replacement-repeated-digest-refused"
    );
}

#[test]
fn replacement_reviewer_and_issuer_must_be_independent_of_route_owner() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    for issuer_is_owner in [false, true] {
        let mut authority = TestReplacementAuthority::current(&inventory, &route);
        if issuer_is_owner {
            authority.authority_id = "migration-owner".to_owned();
        } else {
            authority.reviewer_id = "migration-owner".to_owned();
        }
        assert_eq!(
            ReplacementEvidence::issue(&inventory, &route, &mut authority)
                .unwrap_err()
                .code(),
            "migration-replacement-evidence-issuance-refused"
        );
    }
}

#[test]
fn replacement_evidence_is_one_shot_and_authority_is_revalidated_after_consumption() {
    let inventory = clean_inventory();
    let first_route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &first_route);
    let first = evidence(&inventory, &first_route, &mut authority);
    let replay = evidence(&inventory, &first_route, &mut authority);
    MigrationPlan::build(
        &inventory,
        vec![first_route.clone()],
        vec![first],
        &mut authority,
    )
    .unwrap();
    assert_eq!(
        MigrationPlan::build(&inventory, vec![first_route], vec![replay], &mut authority,)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-replayed"
    );

    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut mutating = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut mutating);
    mutating.mutate_after_consume = true;
    assert_eq!(
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut mutating)
            .unwrap_err()
            .code(),
        "migration-replacement-evidence-consumption-refused"
    );
}

#[test]
fn final_reconciliation_rejects_substituted_consumption_commitment() {
    let inventory = clean_inventory();
    let (mut substituted_plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &substituted_plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    substituted_plan.substitute_replacement_consumption_for_test(sha('0'));
    let decision = RetirementDecision::reconcile_preservation(
        &substituted_plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-evidence-invalid".to_owned())
    );

    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    replacement_authority.now = 2_001;
    let expired = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert!(
        expired
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );
}

#[test]
fn final_reconciliation_requires_the_originating_persistent_replacement_ledger() {
    let inventory = clean_inventory();

    let (plan, original) = plan_with_authority(&inventory);
    let mut dropped = original.detached_without_ledger();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut dropped,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );

    let (plan, mut rolled_back) = plan_with_authority(&inventory);
    rolled_back.rollback_consumption();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut rolled_back,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );

    let (plan, mut revoked) = plan_with_authority(&inventory);
    revoked.revoke();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut revoked,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );

    let (plan, original) = plan_with_authority(&inventory);
    let mut substituted = original.clone();
    substituted.session_id = sha('7');
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut substituted,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-not-current".to_owned())
    );
}

#[test]
fn persistent_replacement_ledger_rejects_cross_adapter_replay() {
    let inventory = clean_inventory();
    let (plan, mut original) = plan_with_authority(&inventory);
    let mut persistent_peer = original.clone();
    let first = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut original,
        "retirement-reviewer-one",
    );
    assert_eq!(
        first.status,
        RetirementStatus::NonAuthoritativePreservationCandidate
    );
    let replay = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut persistent_peer,
        "retirement-reviewer-two",
    );
    assert_eq!(replay.status, RetirementStatus::Blocked);
    assert!(
        replay
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );
}

#[test]
fn concurrent_final_reconciliation_elects_exactly_one_ledger_winner() {
    let inventory = clean_inventory();
    let (plan, authority) = plan_with_authority(&inventory);
    let plan = Arc::new(plan);
    let barrier = Arc::new(Barrier::new(3));
    let handles = ["concurrent-reviewer-one", "concurrent-reviewer-two"]
        .into_iter()
        .map(|reviewer| {
            let inventory = inventory.clone();
            let plan = Arc::clone(&plan);
            let mut authority = authority.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                preservation_decision(
                    plan.as_ref(),
                    "retire-LEGACY-SKILL:old",
                    &inventory,
                    &mut authority,
                    reviewer,
                )
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let decisions = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| {
                decision.status == RetirementStatus::NonAuthoritativePreservationCandidate
            })
            .count(),
        1
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| decision.status == RetirementStatus::Blocked)
            .count(),
        1
    );
    assert!(decisions.iter().any(|decision| {
        decision
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    }));
}

#[test]
fn final_reconciliation_revalidates_replacement_ledger_around_every_consumption() {
    let inventory = clean_inventory();
    let (plan, mut rolled_back) = plan_with_authority(&inventory);
    rolled_back.rollback_after_final_claim = true;
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut rolled_back,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );

    let (plan, mut drift_after_review) = plan_with_authority(&inventory);
    drift_after_review.revoke_on_final_verify_call = Some(2);
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut drift_after_review,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-drifted".to_owned())
    );

    let (plan, mut drift_after_effect) = plan_with_authority(&inventory);
    drift_after_effect.revoke_on_final_verify_call = Some(4);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    let authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    let decision = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &authorization,
        &mut drift_after_effect,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-ledger-drifted".to_owned())
    );
}

#[test]
fn opaque_authority_debug_is_bounded_and_never_echoes_fields() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut replacement_authority = TestReplacementAuthority::current(&inventory, &route);
    replacement_authority.reviewer_id = "opaque-replacement-reviewer-marker".to_owned();
    let replacement = evidence(&inventory, &route, &mut replacement_authority);
    let replacement_debug = format!("{replacement:?}");
    assert_eq!(
        replacement_debug,
        "ReplacementEvidence { contents: \"<redacted>\" }"
    );
    for secret in [
        replacement_authority.reviewer_id.as_str(),
        replacement_authority.session_id.as_str(),
        replacement_authority.nonce_sha256.as_str(),
        replacement_authority.old_result_sha256.as_str(),
    ] {
        assert!(!replacement_debug.contains(secret));
    }

    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    let mut review_authority =
        TestReviewAuthority::current(&inventory, Od009Decision::RequestPhysicalDeletion);
    review_authority.reviewer_id = "opaque-retirement-reviewer-marker".to_owned();
    let review = RetirementReview::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut review_authority,
    )
    .unwrap();
    let review_debug = format!("{review:?}");
    assert_eq!(
        review_debug,
        "RetirementReview { contents: \"<redacted>\" }"
    );
    assert!(!review_debug.contains(&review_authority.reviewer_id));
    assert!(!review_debug.contains(&review_authority.session_id));

    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    effect_authority.principal_id = "opaque-destructive-principal-marker".to_owned();
    let authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    let authorization_debug = format!("{authorization:?}");
    assert_eq!(
        authorization_debug,
        "DestructiveAuthorization { contents: \"<redacted>\" }"
    );
    assert!(!authorization_debug.contains(&effect_authority.principal_id));
    assert!(!authorization_debug.contains(&effect_authority.session_id));

    let plan_debug = format!("{plan:?}");
    assert!(plan_debug.contains("private_replacement_ledger: \"<redacted>\""));
    assert!(!plan_debug.contains(&test_digest(b"observed-old-behavior-result")));
}

#[test]
fn safe_plan_projection_serialization_never_traverses_private_replacement_authority() {
    let inventory = clean_inventory();
    let (mut plan, _authority) = plan_with_authority(&inventory);
    let pristine_projection = plan.projection();
    assert!(plan.verify_projection(&pristine_projection).is_ok());

    let sentinels = plan.inject_private_serialization_sentinels_for_test();
    let projection = plan.projection();
    assert_eq!(projection, pristine_projection);
    let plan_json = serde_json::to_string(&projection).unwrap();
    let target_json = serde_json::to_string(&plan.targets()[0].projection()).unwrap();
    assert!(plan_json.len() <= 4_096);
    assert!(target_json.len() <= 2_048);
    for sentinel in sentinels {
        assert!(
            !plan_json.contains(&sentinel),
            "plan projection leaked {sentinel}"
        );
        assert!(
            !target_json.contains(&sentinel),
            "target projection leaked {sentinel}"
        );
    }
    for forbidden in [
        "reviewer_id",
        "authority_id",
        "authority_session_id",
        "read_session_id",
        "nonce_sha256",
        "evidence_id",
        "attestation_sha256",
        "consumption_sha256",
        "consumption_binding_sha256",
        "false_pass_control_results",
    ] {
        assert!(!plan_json.contains(forbidden));
        assert!(!target_json.contains(forbidden));
    }

    let roundtrip: MigrationPlanProjection = serde_json::from_str(&plan_json).unwrap();
    assert_eq!(roundtrip, projection);
    assert!(plan.verify_projection(&roundtrip).is_ok());
    assert_eq!(
        plan.verify_current(&inventory).unwrap_err().code(),
        "migration-plan-stale"
    );

    let mut substituted_json: Value = serde_json::from_str(&plan_json).unwrap();
    substituted_json["projection_sha256"] = Value::String(sha('0'));
    let substituted: MigrationPlanProjection = serde_json::from_value(substituted_json).unwrap();
    assert_eq!(
        plan.verify_projection(&substituted).unwrap_err().code(),
        "migration-plan-projection-substituted"
    );
}

#[test]
fn unsafe_special_and_hardlinked_inventory_inputs_are_rejected() {
    for (kind, links, path) in [
        (SurfaceFileKind::Symlink, 1, "skills/old"),
        (SurfaceFileKind::Special, 1, "skills/old"),
        (SurfaceFileKind::Regular, 2, "skills/old"),
        (SurfaceFileKind::Regular, 1, "../escape"),
    ] {
        let source = InventorySurface::observed(
            "LEGACY-SKILL:old",
            "skill",
            path,
            sha('a'),
            kind,
            links,
            SurfaceStatus::Active,
            vec![],
            vec![],
            vec![],
            vec![],
        );
        let error = MigrationInventory::new(
            sha('c'),
            sha('d'),
            sha('e'),
            sha('f'),
            vec![
                source,
                surface(
                    "SKILL:current",
                    SurfaceStatus::Active,
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                ),
            ],
        )
        .unwrap_err();
        assert_eq!(error.code(), "migration-surface-input-refused");
    }
}

#[test]
fn final_session_rotation_invalidates_mutate_restore_reuse() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    let mut restored_bytes_new_session = inventory.clone();
    restored_bytes_new_session.rotate_session_for_test(sha('9'));
    assert_eq!(
        plan.verify_current(&restored_bytes_new_session)
            .unwrap_err()
            .code(),
        "migration-plan-stale"
    );
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &restored_bytes_new_session,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert!(
        decision
            .reasons
            .contains(&"migration-replacement-evidence-invalid".to_owned())
    );
}

#[test]
fn clean_target_is_only_a_non_authoritative_preservation_candidate() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut replacement_authority,
        "retirement-reviewer",
    );
    assert_eq!(
        decision.status,
        RetirementStatus::NonAuthoritativePreservationCandidate
    );
    assert_eq!(
        decision.claim_ceiling,
        "migration_candidate_not_adoption_or_retirement"
    );
}

#[test]
fn active_readers_writers_routes_and_generated_authority_block_retirement() {
    let inventory = inventory_with_source(
        surface(
            "LEGACY-SKILL:old",
            SurfaceStatus::Active,
            vec!["reader-a".to_owned()],
            vec!["writer-a".to_owned()],
            vec!["legacy-command".to_owned()],
            vec!["docs/generated/old.json".to_owned()],
        ),
        'f',
    );
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut replacement_authority,
        "retirement-reviewer",
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    for expected in [
        "migration-active-readers-remain",
        "migration-active-writers-remain",
        "migration-public-routes-remain",
        "migration-generated-authority-remains",
    ] {
        assert!(decision.reasons.contains(&expected.to_owned()));
    }
}

#[test]
fn compatibility_usage_or_open_window_blocks_retirement() {
    let inventory = clean_inventory();
    let route = CompatibilityRoute::new(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "SKILL:current",
        "migration-owner",
        "Compatibility warning remains visible",
        sha('1'),
        "version-2-boundary",
        "zero-active-references",
        sha('2'),
        3,
        false,
    );
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    let evidence = evidence(&inventory, &route, &mut authority);
    let plan =
        MigrationPlan::build(&inventory, vec![route], vec![evidence], &mut authority).unwrap();
    let decision = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut authority,
        "retirement-reviewer",
    );
    assert!(
        decision
            .reasons
            .contains(&"migration-compatibility-window-open".to_owned())
    );
}

#[test]
fn route_owner_and_replacement_reviewer_cannot_self_accept_retirement() {
    let inventory = clean_inventory();
    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    for reviewer in ["migration-owner", "replacement-reviewer"] {
        let mut authority =
            TestReviewAuthority::current(&inventory, Od009Decision::PreservePhysicalArtifact);
        authority.reviewer_id = reviewer.to_owned();
        let error =
            RetirementReview::issue(&plan, "retire-LEGACY-SKILL:old", &inventory, &mut authority)
                .unwrap_err();
        assert_eq!(error.code(), "migration-retirement-review-issuance-refused");
    }
}

#[test]
fn destructive_retirement_requires_separate_authority() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let blocked = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert_eq!(blocked.status, RetirementStatus::Blocked);
    assert!(
        blocked
            .reasons
            .contains(&"migration-destructive-authority-required".to_owned())
    );
    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    let authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    let authorized = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &authorization,
        &mut replacement_authority,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(
        authorized.status,
        RetirementStatus::DestructiveRetirementCandidate
    );
}

#[test]
fn retirement_review_issuance_rejects_same_principal_session_expiry_and_stale_binding() {
    let inventory = clean_inventory();
    let (plan, _replacement_authority) = plan_with_authority(&inventory);
    for case in 0..6 {
        let mut authority =
            TestReviewAuthority::current(&inventory, Od009Decision::PreservePhysicalArtifact);
        match case {
            0 => authority.authority_id = authority.reviewer_id.clone(),
            1 => authority.session_id = sha('f'),
            2 => authority.now = authority.expires_at + 1,
            3 => authority.inventory_sha256 = sha('0'),
            4 => authority.authority_id = "migration-owner".to_owned(),
            5 => authority.nonce_sha256 = "caller-nonce".to_owned(),
            _ => unreachable!(),
        }
        assert_review_issue_refused(&plan, &inventory, authority);
    }
}

#[test]
fn stale_substituted_unknown_and_replayed_retirement_reviews_fail_closed() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);

    let (mut substituted_plan, mut plan_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    substituted_plan.substitute_plan_for_test(sha('0'));
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &substituted_plan,
        &mut replacement_authority,
        &mut plan_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-retirement-review-invalid".to_owned())
    );

    let (mut substituted_target, mut target_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    substituted_target.substitute_target_for_test("retire-UNKNOWN");
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &substituted_target,
        &mut replacement_authority,
        &mut target_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);

    let (stale_review, mut stale_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    stale_authority.candidate_id = sha('0');
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &stale_review,
        &mut replacement_authority,
        &mut stale_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);

    let (expired_review, mut expired_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    expired_authority.now = expired_authority.expires_at + 1;
    let expired = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &expired_review,
        &mut replacement_authority,
        &mut expired_authority,
    );
    assert_eq!(expired.status, RetirementStatus::Blocked);

    let (unknown_review, mut unknown_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    let unknown = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-UNKNOWN",
        &inventory,
        &unknown_review,
        &mut replacement_authority,
        &mut unknown_authority,
    );
    assert!(
        unknown
            .reasons
            .contains(&"migration-retirement-target-unknown".to_owned())
    );

    let (review, mut authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    assert_eq!(
        RetirementDecision::reconcile_preservation(
            &plan,
            "retire-LEGACY-SKILL:old",
            &inventory,
            &review,
            &mut replacement_authority,
            &mut authority,
        )
        .status,
        RetirementStatus::NonAuthoritativePreservationCandidate
    );
    let replay = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut authority,
    );
    assert_eq!(replay.status, RetirementStatus::Blocked);
    assert!(
        replay
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );
}

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

#[test]
fn substituted_expired_conflicting_and_replayed_destructive_authorizations_fail_closed() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);

    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    let mut substituted = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    substituted.substitute_effect_scope_for_test(sha('3'));
    let decision = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &substituted,
        &mut replacement_authority,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-destructive-authorization-invalid".to_owned())
    );

    let (expired_review, mut expired_review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut expired_effect =
        TestEffectAuthority::current(&inventory, expired_review.effect_scope_sha256());
    let expired_authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &expired_review,
        &expired_review_authority,
        &mut expired_effect,
    )
    .unwrap();
    expired_effect.now = expired_effect.expires_at + 1;
    let expired = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &expired_review,
        &expired_authorization,
        &mut replacement_authority,
        &mut expired_review_authority,
        &mut expired_effect,
    );
    assert_eq!(expired.status, RetirementStatus::Blocked);

    let (replay_review, mut replay_review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut replay_effect =
        TestEffectAuthority::current(&inventory, replay_review.effect_scope_sha256());
    let replay_authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &replay_review,
        &replay_review_authority,
        &mut replay_effect,
    )
    .unwrap();
    assert_eq!(
        RetirementDecision::reconcile_destructive(
            &plan,
            "retire-LEGACY-SKILL:old",
            &inventory,
            &replay_review,
            &replay_authorization,
            &mut replacement_authority,
            &mut replay_review_authority,
            &mut replay_effect,
        )
        .status,
        RetirementStatus::DestructiveRetirementCandidate
    );
    let replay = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &replay_review,
        &replay_authorization,
        &mut replacement_authority,
        &mut replay_review_authority,
        &mut replay_effect,
    );
    assert_eq!(replay.status, RetirementStatus::Blocked);
    assert!(
        replay
            .reasons
            .contains(&"migration-replacement-ledger-invalid-or-replayed".to_owned())
    );

    let (deletion_review, deletion_review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut conflicting_effect =
        TestEffectAuthority::current(&inventory, deletion_review.effect_scope_sha256());
    let conflicting_authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &deletion_review,
        &deletion_review_authority,
        &mut conflicting_effect,
    )
    .unwrap();
    let (preserve_review, mut preserve_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "second-retirement-reviewer",
    );
    let conflict = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &preserve_review,
        &conflicting_authorization,
        &mut replacement_authority,
        &mut preserve_authority,
        &mut conflicting_effect,
    );
    assert!(
        conflict
            .reasons
            .contains(&"migration-conflicting-destructive-authorization".to_owned())
    );
}

#[test]
fn authority_mutation_during_one_shot_consumption_is_revalidated() {
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::PreservePhysicalArtifact,
        "retirement-reviewer",
    );
    review_authority.mutate_after_consume = true;
    let decision = RetirementDecision::reconcile_preservation(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &mut replacement_authority,
        &mut review_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-retirement-review-invalid-or-replayed".to_owned())
    );

    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let (review, mut review_authority) = issue_review(
        &plan,
        &inventory,
        Od009Decision::RequestPhysicalDeletion,
        "retirement-reviewer",
    );
    let mut effect_authority =
        TestEffectAuthority::current(&inventory, review.effect_scope_sha256());
    let authorization = DestructiveAuthorization::issue(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &review_authority,
        &mut effect_authority,
    )
    .unwrap();
    effect_authority.mutate_after_consume = true;
    let decision = RetirementDecision::reconcile_destructive(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &review,
        &authorization,
        &mut replacement_authority,
        &mut review_authority,
        &mut effect_authority,
    );
    assert_eq!(decision.status, RetirementStatus::Blocked);
    assert!(
        decision
            .reasons
            .contains(&"migration-destructive-authorization-invalid-or-replayed".to_owned())
    );
}

#[test]
fn external_callers_cannot_mint_clone_or_deserialize_replacement_or_retirement_authority() {
    let root = temp_root("public-retirement-seal");
    fs::create_dir_all(root.join("src/bin")).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/migration/mod.rs"),
        root.join("src/migration.rs"),
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "migration-retirement-seal-probe"
version = "0.0.0"
edition = "2024"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"
"#,
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod migration;\n").unwrap();
    fs::write(
        root.join("src/bin/forge.rs"),
        r#"use migration_retirement_seal_probe::migration::{
    DestructiveAuthorization, MigrationPlan, MigrationPlanProjection, ReplacementEvidence,
    RetirementReview, RetirementTarget,
};

fn forge_replacement() {
    let _ = ReplacementEvidence::new(
        "LEGACY-SKILL:old",
        "SKILL:current",
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let _ = MigrationPlan::build;
}

fn forge_review() {
    let _ = RetirementReview::new(
        "caller-reviewer",
        true,
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        true,
        true,
    );
}

fn forge_effect() {
    let _ = DestructiveAuthorization::new();
}

fn clone_review(value: RetirementReview) {
    let _ = value.clone();
}

fn clone_replacement(value: ReplacementEvidence) {
    let _ = value.clone();
}

fn clone_plan(value: MigrationPlan) {
    let _ = value.clone();
}

fn clone_target(value: RetirementTarget) {
    let _ = value.clone();
}

fn require_serializable<T: serde::Serialize>(_: &T) {}
fn require_deserializable<T: for<'de> serde::Deserialize<'de>>() {}

fn deserialize_review() {
    require_deserializable::<ReplacementEvidence>();
    require_deserializable::<RetirementReview>();
    require_deserializable::<DestructiveAuthorization>();
    require_deserializable::<MigrationPlan>();
    require_deserializable::<RetirementTarget>();
}

fn serialize_plan(value: &MigrationPlan) {
    require_serializable(value);
}

fn serialize_target(value: &RetirementTarget) {
    require_serializable(value);
}

fn replay_projection(value: MigrationPlanProjection) {
    let _plan: MigrationPlan = value.into();
}

fn main() {}
"#,
    )
    .unwrap();
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--bin", "forge"])
        .env("CARGO_TARGET_DIR", root.join("target"))
        .current_dir(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "external forgery unexpectedly compiled"
    );
    for expected in [
        "RetirementReview",
        "DestructiveAuthorization",
        "ReplacementEvidence",
        "MigrationPlan",
        "RetirementTarget",
        "build",
        "new",
        "clone",
        "Serialize",
        "Deserialize",
        "From<MigrationPlanProjection>",
    ] {
        assert!(
            stderr.contains(expected),
            "unexpected compile failure: {stderr}"
        );
    }

    let migration_source = root.join("src/migration.rs");
    let mut source = fs::read_to_string(&migration_source).unwrap();
    source.push_str(
        r#"
#[allow(dead_code)]
fn consumed_replacement_trait_probe(value: ConsumedReplacementEvidence) {
    fn require_serializable<T: serde::Serialize>(_: &T) {}
    fn require_deserializable<T: for<'de> serde::Deserialize<'de>>() {}
    let _ = value.clone();
    require_serializable(&value);
    require_deserializable::<ConsumedReplacementEvidence>();
}
"#,
    );
    fs::write(&migration_source, source).unwrap();
    let consumed_output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["check", "--offline", "--quiet", "--lib"])
        .env("CARGO_TARGET_DIR", root.join("target-consumed"))
        .current_dir(&root)
        .output()
        .unwrap();
    let consumed_stderr = String::from_utf8_lossy(&consumed_output.stderr);
    assert!(
        !consumed_output.status.success(),
        "consumed replacement evidence unexpectedly implemented copy/serde traits"
    );
    for expected in [
        "ConsumedReplacementEvidence",
        "clone",
        "Serialize",
        "Deserialize",
    ] {
        assert!(
            consumed_stderr.contains(expected),
            "unexpected consumed-token compile failure: {consumed_stderr}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn renamed_only_or_receipt_only_replacement_is_not_proof() {
    let inventory = clean_inventory();
    let route = route("LEGACY-SKILL:old", "SKILL:current");
    let mut authority = TestReplacementAuthority::current(&inventory, &route);
    authority.new_verdict = EvidenceVerdict::CausalFailure;
    assert_eq!(
        ReplacementEvidence::issue(&inventory, &route, &mut authority)
            .unwrap_err()
            .code(),
        "migration-replacement-observation-invalid"
    );
}

#[test]
fn plan_verify_and_retirement_reconciliation_are_zero_write() {
    let root = temp_root("zero-write");
    fs::write(root.join("sentinel"), b"preserve").unwrap();
    let before = tree(&root);
    let inventory = clean_inventory();
    let (plan, mut replacement_authority) = plan_with_authority(&inventory);
    let _ = plan.verify_current(&inventory);
    let _ = preservation_decision(
        &plan,
        "retire-LEGACY-SKILL:old",
        &inventory,
        &mut replacement_authority,
        "retirement-reviewer",
    );
    assert_eq!(tree(&root), before);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fixture_catalog_covers_security_mutation_and_false_pass_cases() {
    let current: Value = serde_json::from_str(include_str!(
        "../../fixtures/migration-engine/current-valid.json"
    ))
    .unwrap();
    let red: Value = serde_json::from_str(include_str!(
        "../../fixtures/migration-engine/red-cases.json"
    ))
    .unwrap();
    assert_eq!(current["schema_version"], "MigrationEngineFixture-v1");
    let cases = red["cases"].as_array().unwrap();
    assert!(cases.len() >= 55);
    for expected in [
        "mutate-restore-session",
        "active-reader",
        "active-writer",
        "renamed-only-replacement",
        "caller-supplied-replacement-digest",
        "forged-replacement-evidence",
        "replacement-evidence-clone",
        "replacement-evidence-deserialize",
        "missing-named-false-pass-control",
        "repeated-hash-evidence",
        "replacement-evidence-replay",
        "replacement-evidence-mutate-restore",
        "same-route-owner-reviewer",
        "stale-replacement-evidence-binding",
        "replacement-ledger-loss",
        "replacement-ledger-rollback",
        "replacement-evidence-revocation",
        "replacement-authority-substitution",
        "replacement-cross-process-replay",
        "replacement-finalization-concurrent-winner",
        "replacement-post-check-drift",
        "opaque-token-debug-echo",
        "consumed-evidence-clone",
        "consumed-evidence-serialize",
        "migration-plan-clone",
        "migration-plan-serialize",
        "migration-plan-deserialize",
        "migration-plan-projection-replay",
        "migration-plan-roundtrip-authority",
        "migration-plan-projection-substitution",
        "symlink-surface",
        "hardlink-surface",
        "special-file-surface",
        "unauthorized-deletion",
        "caller-asserted-independent-boolean",
        "caller-asserted-destructive-authority",
        "forged-retirement-review",
        "forged-destructive-authorization",
        "same-review-effect-principal",
        "same-review-effect-session",
        "expired-retirement-review",
        "stale-retirement-binding",
        "substituted-effect-scope",
        "retirement-authorization-replay",
        "conflicting-od009-decision",
        "authority-mutation-during-consumption",
    ] {
        assert!(cases.iter().any(|case| case == expected));
    }
}

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "hul-migration-043-{label}-{}-{}",
        std::process::id(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(root: &Path, current: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    walk(root, root, &mut out);
    out
}
