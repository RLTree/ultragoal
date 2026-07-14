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
