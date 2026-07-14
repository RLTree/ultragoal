#[test]
fn every_host_surface_and_reader_provenance_is_reobserved_after_capture() {
    let fixture = Fixture::new("post-capture-all-surfaces");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 9);
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
    let before_tree = fixture.tree();

    for surface in [
        "marketplace",
        "cache",
        "app_registry",
        "plugins_ui",
        "discovery",
        "runtime",
        "provenance",
    ] {
        let before = Reader::complete(&bundle, &host, session.binding());
        let mut after = before.clone();
        let canary = format!("POST_CAPTURE_{surface}_CANARY");
        match surface {
            "marketplace" => after.marketplace = Some(canary.as_bytes().to_vec()),
            "cache" => after.cache = Some(canary.as_bytes().to_vec()),
            "app_registry" | "discovery" => after.registry = Some(canary.as_bytes().to_vec()),
            "plugins_ui" => after.ui = Some(canary.as_bytes().to_vec()),
            "runtime" => after.runtime = None,
            "provenance" => {
                after.provenance_sha256 =
                    "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into()
            }
            _ => unreachable!(),
        }
        let mut reader = PostCaptureReader {
            before,
            after,
            surface,
            marketplace_reads: 0,
            cache_reads: 0,
            registry_reads: 0,
            ui_reads: 0,
            runtime_reads: 0,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert!(matches!(
            error.id(),
            HostLifecycleErrorId::ObservationChanged
                | HostLifecycleErrorId::ObservationConflict
                | HostLifecycleErrorId::ObservationUnavailable
                | HostLifecycleErrorId::UnsupportedSubstitution
        ));
        assert!(!error.to_string().contains(&canary));
        assert_eq!(fixture.tree(), before_tree, "{surface} drift wrote");
    }

    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before_tree);
}

struct TransientMarketplaceReader {
    stable: Reader,
    mutation: Option<Vec<u8>>,
    marketplace_reads: usize,
}

impl HostSurfaceReader for TransientMarketplaceReader {
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

impl TestSurfaceSource for TransientMarketplaceReader {
    fn provenance_sha256(&self) -> &str {
        &self.stable.provenance_sha256
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if self.marketplace_reads == 3 {
            return Ok(self.mutation.clone());
        }
        let _ = maximum;
        self.stable.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.stable.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.stable.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.stable.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.stable.read_runtime_raw()
    }
}

#[test]
fn post_capture_disappear_reappear_and_mutate_restore_are_refused_without_write() {
    let fixture = Fixture::new("post-capture-transient-drift");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 10);
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
    let before_tree = fixture.tree();
    for mutation in [None, Some(b"MUTATE_RESTORE_CANARY".to_vec())] {
        let mut reader = TransientMarketplaceReader {
            stable: Reader::complete(&bundle, &host, session.binding()),
            mutation,
            marketplace_reads: 0,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ObservationChanged);
        assert!(!error.to_string().contains("MUTATE_RESTORE_CANARY"));
        assert_eq!(fixture.tree(), before_tree);
    }
    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before_tree);
}

struct InstalledDriftReader {
    inner: Reader,
    installed_path: std::path::PathBuf,
    replacement: Vec<u8>,
    marketplace_reads: usize,
}

impl HostSurfaceReader for InstalledDriftReader {
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
