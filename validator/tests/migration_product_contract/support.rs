use crate::migration::product::{
    derive_product_plan, AdoptedRegistrySnapshot, ApplyAuthorizationAuthority, AuthoritySnapshot,
    ConfinedMigrationEffect, DurableMigrationStore, EffectFault, EffectObservation, JournalPhase,
    MigrationInputBinding, MigrationInputSource, MigrationOperation, PlanDisposition,
    PlannedMigrationEffect, ProductInputSnapshot, ProductMigrationError, ProductMigrationPlan,
    ReservationRequest, ReservationResult, StoreFault,
};
use crate::migration::{InventorySurface, MigrationInventory, SurfaceFileKind, SurfaceStatus};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

pub(crate) fn sha(byte: char) -> String {
    format!("sha256:{}", byte.to_string().repeat(64))
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

pub(crate) fn surface(
    id: &str,
    kind: &str,
    path: &str,
    digest_byte: char,
    status: SurfaceStatus,
    readers: &[&str],
    writers: &[&str],
    routes: &[&str],
    generated: &[&str],
) -> InventorySurface {
    InventorySurface::observed(
        id,
        kind,
        path,
        sha(digest_byte),
        SurfaceFileKind::Regular,
        1,
        status,
        readers.iter().map(|value| (*value).to_owned()).collect(),
        writers.iter().map(|value| (*value).to_owned()).collect(),
        routes.iter().map(|value| (*value).to_owned()).collect(),
        generated.iter().map(|value| (*value).to_owned()).collect(),
    )
}

pub(crate) fn inventory(surfaces: Vec<InventorySurface>, session: char) -> MigrationInventory {
    MigrationInventory::new(sha('c'), sha('d'), sha('e'), sha(session), surfaces).unwrap()
}

pub(crate) fn false_pass_controls() -> Value {
    json!({
        "proof-artifact": sha('1'),
        "receipt-production": sha('2'),
        "score-only": sha('3'),
        "test-manipulation": sha('4'),
        "verbosity": sha('5')
    })
}

pub(crate) fn route(
    route_id: &str,
    source: &str,
    source_path: &str,
    target: &str,
    source_digest: char,
    target_digest: char,
    disposition: Option<&str>,
) -> Value {
    let (transition, adoption) = match disposition {
        Some("compatibility") => (
            json!({
                "compatibility_behavior":"exact-route-only",
                "compatibility_boundary":"explicit-only",
                "replacement_state":"verified",
                "active_reader_writer_state":"none",
                "observed_authority_state":"compatibility-route-retained",
                "equivalence_proof":"executed-behavior-v1",
                "physical_cleanup_state":"preserve",
                "proof_refs":[]
            }),
            Some(json!({
                "schema_version":"MigrationTransitionAdoption-v1",
                "disposition":"compatibility",
                "source_digest_sha256":sha(source_digest),
                "canonical_target_digest_sha256":sha(target_digest),
                "post_status":"context_only",
                "exact_active_readers":[],
                "exact_active_writers":[],
                "exact_public_routes":[route_id],
                "exact_generated_outputs":[],
                "behavior_execution_kind":"live-behavior-execution-v1",
                "behavior_execution_sha256":sha('6'),
                "rollback_execution_sha256":sha('7'),
                "false_pass_control_sha256":false_pass_controls(),
                "compatibility_prerequisites":{
                    "schema_version":"CompatibilityPrerequisites-v1",
                    "owner_id":"maintenance-owner",
                    "semantic_target_id":target,
                    "user_facing_warning":"This compatibility route is deprecated; use the canonical target.",
                    "usage_measurement":{
                        "schema_version":"CompatibilityUsageMeasurement-v1",
                        "route_id":route_id,
                        "metric":"legacy-route-invocations",
                        "evidence_sha256":sha('d'),
                        "window_start_unix_ms":1_000,
                        "window_end_unix_ms":2_000,
                        "observed_invocations":12
                    },
                    "boundary":{
                        "schema_version":"CompatibilityBoundary-v1",
                        "deadline_unix_ms":86_402_000
                    },
                    "removal_condition":{
                        "schema_version":"CompatibilityRemovalCondition-v1",
                        "metric":"legacy-route-invocations",
                        "operator":"less-than-or-equal",
                        "threshold":0,
                        "required_consecutive_windows":2
                    }
                },
                "preserve_physical_bytes":true
            })),
        ),
        Some("retirement") => (
            json!({
                "compatibility_behavior":"removed",
                "compatibility_boundary":"closed",
                "replacement_state":"verified",
                "active_reader_writer_state":"none",
                "observed_authority_state":"retired",
                "equivalence_proof":"executed-behavior-v1",
                "physical_cleanup_state":"preserve",
                "proof_refs":[]
            }),
            Some(json!({
                "schema_version":"MigrationTransitionAdoption-v1",
                "disposition":"retirement",
                "source_digest_sha256":sha(source_digest),
                "canonical_target_digest_sha256":sha(target_digest),
                "post_status":"retired",
                "exact_active_readers":[],
                "exact_active_writers":[],
                "exact_public_routes":[],
                "exact_generated_outputs":[],
                "behavior_execution_kind":"live-behavior-execution-v1",
                "behavior_execution_sha256":sha('8'),
                "rollback_execution_sha256":sha('9'),
                "false_pass_control_sha256":false_pass_controls(),
                "preserve_physical_bytes":true
            })),
        ),
        None => (
            json!({
                "compatibility_behavior":"unverified",
                "compatibility_boundary":"blocked-by-OD-008",
                "replacement_state":"unverified",
                "active_reader_writer_state":"active",
                "observed_authority_state":"active",
                "equivalence_proof":"missing",
                "physical_cleanup_state":"blocked-by-OD-009",
                "proof_refs":[]
            }),
            None,
        ),
        _ => unreachable!(),
    };
    let mut transition = transition;
    if let Some(adoption) = adoption {
        transition["adopted_effect"] = adoption;
    }
    json!({
        "route_id":route_id,
        "match":{"stable_id":source,"kind":"legacy-skill","relative_path":source_path},
        "canonical_target":target,
        "intended_disposition":"non-authoritative",
        "transition":transition
    })
}

pub(crate) fn registry_bytes(routes: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version":"AuthorityRoutingRegistry-v1",
        "contract_id":"harness-ultragoal-successor-contract-v2",
        "destructive_cleanup_authorized":false,
        "authority_rule":"Only exact adopted machine transitions may execute; prose and receipts are not retirement proof.",
        "routes":routes
    }))
    .unwrap()
}

pub(crate) fn input_with_routes(
    surfaces: Vec<InventorySurface>,
    routes: Vec<Value>,
    session: char,
) -> ProductInputSnapshot {
    let registry = AdoptedRegistrySnapshot::observed(
        "migration/authority-routes.json",
        SurfaceFileKind::Regular,
        1,
        sha('f'),
        registry_bytes(routes),
    )
    .unwrap();
    ProductInputSnapshot::observed(inventory(surfaces, session), registry).unwrap()
}

pub(crate) fn compatibility_route() -> Value {
    route(
        "route-old-to-current",
        "LEGACY-SKILL:old",
        "skills/old/SKILL.md",
        "SKILL:current",
        'a',
        'b',
        Some("compatibility"),
    )
}

pub(crate) fn compatibility_input_with_route(
    compatibility_route: Value,
    session: char,
) -> ProductInputSnapshot {
    input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Active,
                &["legacy-reader"],
                &["legacy-writer"],
                &["legacy-public"],
                &["legacy-generated"],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &[],
                &[],
            ),
        ],
        vec![compatibility_route],
        session,
    )
}

pub(crate) fn compatibility_input() -> ProductInputSnapshot {
    compatibility_input_with_route(compatibility_route(), '0')
}

pub(crate) fn retirement_input() -> ProductInputSnapshot {
    input_with_routes(
        vec![
            surface(
                "LEGACY-SKILL:old",
                "legacy-skill",
                "skills/old/SKILL.md",
                'a',
                SurfaceStatus::Candidate,
                &["legacy-reader"],
                &["legacy-writer"],
                &["legacy-public"],
                &["legacy-generated"],
            ),
            surface(
                "SKILL:current",
                "skill",
                "skills/current/SKILL.md",
                'b',
                SurfaceStatus::Active,
                &[],
                &[],
                &["current-public"],
                &[],
            ),
        ],
        vec![route(
            "route-old-to-current",
            "LEGACY-SKILL:old",
            "skills/old/SKILL.md",
            "SKILL:current",
            'a',
            'b',
            Some("retirement"),
        )],
        '0',
    )
}

#[derive(Clone)]
pub(crate) struct FakeSource {
    pub(crate) input: ProductInputSnapshot,
    pub(crate) stale: bool,
}

impl FakeSource {
    pub(crate) fn new(input: ProductInputSnapshot) -> Self {
        Self {
            input,
            stale: false,
        }
    }
}

impl MigrationInputSource for FakeSource {
    fn capture(&mut self) -> Result<ProductInputSnapshot, ProductMigrationError> {
        Ok(self.input.clone())
    }

    fn revalidate(
        &mut self,
        binding: &MigrationInputBinding,
        _applied_effect_ids: &[String],
    ) -> Result<(), ProductMigrationError> {
        if self.stale || &self.input.binding() != binding {
            Err(ProductMigrationError::new("test-migration-source-stale"))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub(crate) struct FakeAuthority {
    pub(crate) principal: String,
    pub(crate) authority: String,
    pub(crate) session: String,
    pub(crate) nonce: String,
    pub(crate) issued: u64,
    pub(crate) expires: u64,
    pub(crate) now: u64,
    pub(crate) input_binding: String,
    pub(crate) plan_sha256: String,
    pub(crate) key: String,
    pub(crate) boundary_authority: String,
    pub(crate) boundary_source_identity: String,
    pub(crate) boundary_sequence: u64,
    pub(crate) boundary_now: u64,
    pub(crate) current_product_version: String,
    pub(crate) boundary_key: String,
    pub(crate) boundary_capture_count: Arc<AtomicUsize>,
    pub(crate) boundary_cross_after_captures: Option<usize>,
    pub(crate) boundary_crossed_now: Option<u64>,
    pub(crate) boundary_crossed_version: Option<String>,
}

impl FakeAuthority {
    pub(crate) fn boundary() -> Self {
        Self {
            principal: "migration-operator".to_owned(),
            authority: "root-migration-authority".to_owned(),
            session: sha('a'),
            nonce: sha('0'),
            issued: 100,
            expires: 200,
            now: 150,
            input_binding: sha('0'),
            plan_sha256: sha('1'),
            key: "test-seal-key".to_owned(),
            boundary_authority: "root-compatibility-boundary-authority".to_owned(),
            boundary_source_identity: sha('b'),
            boundary_sequence: 1,
            boundary_now: 10_000,
            current_product_version: "0.0.12".to_owned(),
            boundary_key: "test-boundary-seal-key".to_owned(),
            boundary_capture_count: Arc::new(AtomicUsize::new(0)),
            boundary_cross_after_captures: None,
            boundary_crossed_now: None,
            boundary_crossed_version: None,
        }
    }

    pub(crate) fn current(plan: &ProductMigrationPlan, nonce: char) -> Self {
        let mut value = Self::boundary();
        value.nonce = sha(nonce);
        value.input_binding = plan.input_binding().binding_sha256().to_owned();
        value.plan_sha256 = plan.plan_sha256().to_owned();
        value.boundary_sequence = 2;
        value
    }

    fn seal_for(&self, binding: &str) -> String {
        hash(format!("{}|{}", self.key, binding).as_bytes())
    }

    fn boundary_seal_for(&self, binding: &str) -> String {
        hash(format!("{}|{}", self.boundary_key, binding).as_bytes())
    }

    fn current_boundary_binding(&self) -> String {
        let capture_count = self.boundary_capture_count.load(Ordering::SeqCst);
        let crossed = self
            .boundary_cross_after_captures
            .is_some_and(|captures| capture_count >= captures);
        let observed_at_unix_ms = if crossed {
            self.boundary_crossed_now.unwrap_or(self.boundary_now)
        } else {
            self.boundary_now
        };
        let current_product_version = if crossed {
            self.boundary_crossed_version
                .as_deref()
                .unwrap_or(&self.current_product_version)
        } else {
            &self.current_product_version
        };
        hash(
            format!(
                "migration-compatibility-boundary-observation-binding-v1|{}|{}|{}|{}|{}",
                self.boundary_authority,
                self.boundary_source_identity,
                self.boundary_sequence.saturating_add(capture_count as u64),
                observed_at_unix_ms,
                current_product_version,
            )
            .as_bytes(),
        )
    }

    pub(crate) fn cross_deadline_after_captures(&mut self, captures: usize, now: u64) {
        self.boundary_cross_after_captures = Some(captures);
        self.boundary_crossed_now = Some(now);
    }

    pub(crate) fn cross_version_after_captures(&mut self, captures: usize, version: &str) {
        self.boundary_cross_after_captures = Some(captures);
        self.boundary_crossed_version = Some(version.to_owned());
    }

    fn boundary_crossed(&self) -> bool {
        self.boundary_cross_after_captures
            .is_some_and(|captures| self.boundary_capture_count.load(Ordering::SeqCst) >= captures)
    }
}

pub(crate) fn derive_plan(
    input: &ProductInputSnapshot,
) -> Result<ProductMigrationPlan, ProductMigrationError> {
    let authority = FakeAuthority::boundary();
    derive_product_plan(input, Some(&authority))
}

impl ApplyAuthorizationAuthority for FakeAuthority {
    fn principal_id(&self) -> &str {
        &self.principal
    }
    fn authority_id(&self) -> &str {
        &self.authority
    }
    fn session_id(&self) -> &str {
        &self.session
    }
    fn nonce_sha256(&self) -> &str {
        &self.nonce
    }
    fn issued_at_unix_ms(&self) -> u64 {
        self.issued
    }
    fn expires_at_unix_ms(&self) -> u64 {
        self.expires
    }
    fn now_unix_ms(&self) -> u64 {
        self.now
    }
    fn current_binding(&self) -> (&str, &str) {
        (&self.input_binding, &self.plan_sha256)
    }
    fn seal(&mut self, binding_sha256: &str) -> Result<String, ProductMigrationError> {
        Ok(self.seal_for(binding_sha256))
    }
    fn verify_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool {
        self.seal_for(binding_sha256) == seal_sha256
    }
    fn compatibility_boundary_authority_id(&self) -> &str {
        &self.boundary_authority
    }
    fn compatibility_boundary_source_identity_sha256(&self) -> &str {
        &self.boundary_source_identity
    }
    fn compatibility_boundary_observation_sequence(&self) -> u64 {
        self.boundary_sequence
            .saturating_add(self.boundary_capture_count.load(Ordering::SeqCst) as u64)
    }
    fn compatibility_boundary_observed_at_unix_ms(&self) -> u64 {
        if self.boundary_crossed() {
            self.boundary_crossed_now.unwrap_or(self.boundary_now)
        } else {
            self.boundary_now
        }
    }
    fn compatibility_boundary_current_product_version(&self) -> &str {
        if self.boundary_crossed() {
            self.boundary_crossed_version
                .as_deref()
                .unwrap_or(&self.current_product_version)
        } else {
            &self.current_product_version
        }
    }
    fn seal_compatibility_boundary(
        &self,
        binding_sha256: &str,
    ) -> Result<String, ProductMigrationError> {
        Ok(self.boundary_seal_for(binding_sha256))
    }
    fn verify_compatibility_boundary_seal(&self, binding_sha256: &str, seal_sha256: &str) -> bool {
        let valid = self.boundary_seal_for(binding_sha256) == seal_sha256;
        if valid && binding_sha256 == self.current_boundary_binding() {
            self.boundary_capture_count.fetch_add(1, Ordering::SeqCst);
        }
        valid
    }
}

#[derive(Default)]
struct StoreState {
    authorizations: BTreeMap<String, crate::migration::product::AuthorizationRecord>,
    consumed: BTreeSet<String>,
    reservations: BTreeMap<String, String>,
    operations: BTreeMap<String, MigrationOperation>,
    cas_count: usize,
    fail_on_cas: Option<usize>,
}

#[derive(Default)]
pub(crate) struct FakeStore {
    state: Mutex<StoreState>,
}

impl FakeStore {
    pub(crate) fn fail_on_cas(&self, number: usize) {
        self.state.lock().unwrap().fail_on_cas = Some(number);
    }

    pub(crate) fn operation(&self, id: &str) -> Option<MigrationOperation> {
        self.state.lock().unwrap().operations.get(id).cloned()
    }

    pub(crate) fn operation_count(&self) -> usize {
        self.state.lock().unwrap().operations.len()
    }

    pub(crate) fn only_operation(&self) -> MigrationOperation {
        let state = self.state.lock().unwrap();
        assert_eq!(state.operations.len(), 1);
        state.operations.values().next().unwrap().clone()
    }

    pub(crate) fn substitute_only_operation_from_json(&self, value: Value) {
        let operation: MigrationOperation = serde_json::from_value(value).unwrap();
        let mut state = self.state.lock().unwrap();
        assert_eq!(state.operations.len(), 1);
        let operation_id = state.operations.keys().next().unwrap().clone();
        state.operations.insert(operation_id, operation);
    }
}

impl DurableMigrationStore for FakeStore {
    fn register_authorization(
        &self,
        record: &crate::migration::product::AuthorizationRecord,
    ) -> Result<(), StoreFault> {
        if !record.validate_shape() {
            return Err(StoreFault::new("test-authorization-invalid"));
        }
        let mut state = self.state.lock().unwrap();
        match state.authorizations.get(record.authorization_id()) {
            Some(existing) if existing == record => Ok(()),
            Some(_) => Err(StoreFault::new("test-authorization-conflict")),
            None => {
                state
                    .authorizations
                    .insert(record.authorization_id().to_owned(), record.clone());
                Ok(())
            }
        }
    }

    fn reserve_once(
        &self,
        request: &ReservationRequest,
        initial: &MigrationOperation,
    ) -> Result<ReservationResult, StoreFault> {
        if !request.validate_shape() || !initial.validate_shape() {
            return Err(StoreFault::new("test-reservation-invalid"));
        }
        let mut state = self.state.lock().unwrap();
        let registered = state
            .authorizations
            .get(request.authorization().authorization_id())
            .ok_or_else(|| StoreFault::new("test-authorization-unregistered"))?;
        if registered != request.authorization() {
            return Err(StoreFault::new("test-authorization-substituted"));
        }
        if state
            .consumed
            .contains(request.authorization().authorization_id())
        {
            return state
                .operations
                .get(request.operation_id())
                .cloned()
                .map(ReservationResult::Existing)
                .ok_or_else(|| StoreFault::new("test-authorization-replayed"));
        }
        for key in request.semantic_keys() {
            if state
                .reservations
                .get(key)
                .is_some_and(|owner| owner != request.operation_id())
            {
                return Err(StoreFault::new("test-semantic-reservation-conflict"));
            }
        }
        state
            .consumed
            .insert(request.authorization().authorization_id().to_owned());
        for key in request.semantic_keys() {
            state
                .reservations
                .insert(key.clone(), request.operation_id().to_owned());
        }
        state
            .operations
            .insert(request.operation_id().to_owned(), initial.clone());
        Ok(ReservationResult::Created(initial.clone()))
    }

    fn load_operation(&self, operation_id: &str) -> Result<Option<MigrationOperation>, StoreFault> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .operations
            .get(operation_id)
            .cloned())
    }

    fn compare_and_swap(
        &self,
        operation_id: &str,
        expected_revision: u64,
        expected_journal_sha256: &str,
        next: &MigrationOperation,
    ) -> Result<MigrationOperation, StoreFault> {
        let mut state = self.state.lock().unwrap();
        state.cas_count += 1;
        if state.fail_on_cas == Some(state.cas_count) {
            state.fail_on_cas = None;
            return Err(StoreFault::new("test-injected-crash"));
        }
        let current = state
            .operations
            .get(operation_id)
            .ok_or_else(|| StoreFault::new("test-operation-missing"))?;
        if current.revision() != expected_revision
            || current.journal_sha256() != expected_journal_sha256
            || next.operation_id() != operation_id
            || next.revision() != expected_revision + 1
            || !next.validate_shape()
        {
            return Err(StoreFault::new("test-cas-conflict"));
        }
        state
            .operations
            .insert(operation_id.to_owned(), next.clone());
        Ok(next.clone())
    }
}

#[derive(Default)]
struct EffectState {
    authorities: BTreeMap<String, AuthoritySnapshot>,
    effect_permit_sha256: BTreeMap<String, Option<String>>,
    apply_count: usize,
    rollback_count: usize,
    reject_effect_id: Option<String>,
    reject_after_effect_id: Option<String>,
    ambiguous_effect_id: Option<String>,
    compatibility_prerequisite_inputs: Vec<String>,
}

#[derive(Clone, Default)]
pub(crate) struct FakeEffects {
    state: Arc<Mutex<EffectState>>,
}

impl FakeEffects {
    pub(crate) fn for_plan(plan: &ProductMigrationPlan) -> Self {
        let authorities = plan
            .effects()
            .iter()
            .map(|effect| (effect.effect_id().to_owned(), effect.before().clone()))
            .collect();
        let effect_permit_sha256 = plan
            .effects()
            .iter()
            .map(|effect| (effect.effect_id().to_owned(), None))
            .collect();
        Self {
            state: Arc::new(Mutex::new(EffectState {
                authorities,
                effect_permit_sha256,
                ..EffectState::default()
            })),
        }
    }

    pub(crate) fn reject(&self, effect_id: &str) {
        self.state.lock().unwrap().reject_effect_id = Some(effect_id.to_owned());
    }

    pub(crate) fn reject_after_effect(&self, effect_id: &str) {
        self.state.lock().unwrap().reject_after_effect_id = Some(effect_id.to_owned());
    }

    pub(crate) fn ambiguous(&self, effect_id: &str) {
        self.state.lock().unwrap().ambiguous_effect_id = Some(effect_id.to_owned());
    }

    pub(crate) fn substitute_effect_permit(&self, effect_id: &str, permit: Option<String>) {
        self.state
            .lock()
            .unwrap()
            .effect_permit_sha256
            .insert(effect_id.to_owned(), permit);
    }

    pub(crate) fn counts(&self) -> (usize, usize) {
        let state = self.state.lock().unwrap();
        (state.apply_count, state.rollback_count)
    }

    pub(crate) fn compatibility_prerequisite_inputs(&self) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .compatibility_prerequisite_inputs
            .clone()
    }

    pub(crate) fn authority(&self, effect_id: &str) -> AuthoritySnapshot {
        self.state
            .lock()
            .unwrap()
            .authorities
            .get(effect_id)
            .unwrap()
            .clone()
    }

    fn observation(
        authority: AuthoritySnapshot,
        effect_permit_sha256: Option<String>,
    ) -> Result<EffectObservation, EffectFault> {
        EffectObservation::live(authority, sha('c'), effect_permit_sha256)
            .map_err(|_| EffectFault::ambiguous("test-observation-invalid"))
    }
}

impl ConfinedMigrationEffect for FakeEffects {
    fn observe(
        &mut self,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        let state = self.state.lock().unwrap();
        let authority = state
            .authorities
            .get(effect.effect_id())
            .cloned()
            .ok_or_else(|| EffectFault::ambiguous("test-effect-unknown"))?;
        let permit = state
            .effect_permit_sha256
            .get(effect.effect_id())
            .cloned()
            .flatten();
        Self::observation(authority, permit)
    }

    fn apply(
        &mut self,
        _operation_id: &str,
        effect: &PlannedMigrationEffect,
        compatibility_effect_permit_sha256: Option<&str>,
    ) -> Result<EffectObservation, EffectFault> {
        let mut state = self.state.lock().unwrap();
        if state.ambiguous_effect_id.as_deref() == Some(effect.effect_id()) {
            return Err(EffectFault::ambiguous("test-effect-ambiguous"));
        }
        if state.reject_effect_id.as_deref() == Some(effect.effect_id()) {
            return Err(EffectFault::rejected("test-effect-rejected"));
        }
        if state.reject_after_effect_id.as_deref() == Some(effect.effect_id()) {
            state.apply_count += 1;
            state
                .authorities
                .insert(effect.effect_id().to_owned(), effect.after().clone());
            state.effect_permit_sha256.insert(
                effect.effect_id().to_owned(),
                compatibility_effect_permit_sha256.map(ToOwned::to_owned),
            );
            return Err(EffectFault::rejected("test-effect-rejected-after-effect"));
        }
        state.apply_count += 1;
        if let Some(prerequisites_sha256) = effect.compatibility_prerequisites_sha256() {
            state
                .compatibility_prerequisite_inputs
                .push(prerequisites_sha256.to_owned());
        }
        state
            .authorities
            .insert(effect.effect_id().to_owned(), effect.after().clone());
        state.effect_permit_sha256.insert(
            effect.effect_id().to_owned(),
            compatibility_effect_permit_sha256.map(ToOwned::to_owned),
        );
        Self::observation(
            effect.after().clone(),
            compatibility_effect_permit_sha256.map(ToOwned::to_owned),
        )
    }

    fn rollback(
        &mut self,
        _operation_id: &str,
        effect: &PlannedMigrationEffect,
    ) -> Result<EffectObservation, EffectFault> {
        let mut state = self.state.lock().unwrap();
        state.rollback_count += 1;
        state
            .authorities
            .insert(effect.effect_id().to_owned(), effect.before().clone());
        state
            .effect_permit_sha256
            .insert(effect.effect_id().to_owned(), None);
        Self::observation(effect.before().clone(), None)
    }
}
