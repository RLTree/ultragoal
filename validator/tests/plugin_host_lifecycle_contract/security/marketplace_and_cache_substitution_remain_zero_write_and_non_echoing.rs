#[test]
fn marketplace_and_cache_substitution_remain_zero_write_and_non_echoing() {
    let fixture = Fixture::new("observation-marketplace-cache-substitution");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 8);
    let repeat = lifecycle(
        &state,
        request(
            LifecycleIntent::RepeatUse,
            Some(bundle.authority.clone()),
            None,
            state.installed.as_ref(),
            false,
            false,
        ),
    );
    let (mut session, host) = fixture.session(&bundle, repeat);
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    let marketplace_canary = "attacker-marketplace-canary";
    let mut marketplace = Reader::complete(&bundle, &host, session.binding());
    marketplace.marketplace = Some(
        crate::host_fixture::marketplace_plan_named(&bundle, marketplace_canary)
            .replacement()
            .to_vec(),
    );
    let marketplace_error = session.capture_and_verify(&mut marketplace).unwrap_err();
    assert_eq!(
        marketplace_error.id(),
        HostLifecycleErrorId::ObservationConflict
    );
    assert!(!marketplace_error.to_string().contains(marketplace_canary));
    assert_eq!(fixture.tree(), before);

    let cache_canary = "attacker-cache-canary";
    let mut cache = Reader::complete(&bundle, &host, session.binding());
    let mut cache_document: serde_json::Value =
        serde_json::from_slice(cache.cache.as_ref().unwrap()).unwrap();
    cache_document["entries"][0]["marketplace"] = serde_json::json!(cache_canary);
    cache.cache = Some(serde_json::to_vec(&cache_document).unwrap());
    let cache_error = session.capture_and_verify(&mut cache).unwrap_err();
    assert_eq!(cache_error.id(), HostLifecycleErrorId::ObservationConflict);
    assert!(!cache_error.to_string().contains(cache_canary));
    assert_eq!(fixture.tree(), before);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before);
}

struct PostCaptureReader {
    before: Reader,
    after: Reader,
    surface: &'static str,
    marketplace_reads: usize,
    cache_reads: usize,
    registry_reads: usize,
    ui_reads: usize,
    runtime_reads: usize,
}

impl PostCaptureReader {
    fn use_after(surface: &str, expected: &str, reads: usize) -> bool {
        surface == expected && reads > 2
    }
}

impl HostSurfaceReader for PostCaptureReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        with_test_transaction(self, request, operation)
    }
}

impl TestSurfaceSource for PostCaptureReader {
    fn provenance_sha256(&self) -> &str {
        if Self::use_after(self.surface, "provenance", self.marketplace_reads) {
            &self.after.provenance_sha256
        } else {
            &self.before.provenance_sha256
        }
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if Self::use_after(self.surface, "marketplace", self.marketplace_reads) {
            let _ = maximum;
            self.after.read_marketplace_raw()
        } else {
            let _ = maximum;
            self.before.read_marketplace_raw()
        }
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.cache_reads += 1;
        if Self::use_after(self.surface, "cache", self.cache_reads) {
            let _ = maximum;
            self.after.read_cache_raw()
        } else {
            let _ = maximum;
            self.before.read_cache_raw()
        }
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.registry_reads += 1;
        if Self::use_after(self.surface, self.surface, self.registry_reads)
            && matches!(self.surface, "app_registry" | "discovery")
        {
            let _ = maximum;
            self.after.read_registry_raw()
        } else {
            let _ = maximum;
            self.before.read_registry_raw()
        }
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.ui_reads += 1;
        if Self::use_after(self.surface, "plugins_ui", self.ui_reads) {
            let _ = maximum;
            self.after.read_plugins_ui_raw()
        } else {
            let _ = maximum;
            self.before.read_plugins_ui_raw()
        }
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.runtime_reads += 1;
        if Self::use_after(self.surface, "runtime", self.runtime_reads) {
            self.after.read_runtime_raw()
        } else {
            self.before.read_runtime_raw()
        }
    }
}
