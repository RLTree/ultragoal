#[test]
fn scope_mutation_after_preflight_is_rejected_before_transaction_metadata_or_reads() {
    let fixture = Fixture::new("preflight-to-transaction-scope-race");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 23);
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
    let host = fixture.host();
    let mut session = fixture.session_with_scope(
        &bundle,
        repeat,
        host.clone(),
        HostScopeAuthority::Repository {
            repository_root: fixture.project.to_string_lossy().into_owned(),
            marketplace: "local-harness-plugins".into(),
        },
    );
    session.apply_confined(&state).unwrap();
    let project = fixture.project.clone();
    let moved = fixture.root.join("transaction-race-original-project");
    let moved_for_callback = moved.clone();
    let mut reader = CountingMutationReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
        counts: AdapterInvocationCounts::default(),
        before_transaction: Some(Box::new(move || {
            std::fs::rename(&project, &moved_for_callback).unwrap();
            std::fs::create_dir(&project).unwrap();
        })),
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
    assert_eq!(reader.counts.transactions.get(), 1);
    assert_eq!(reader.counts.provenance.get(), 0);
    assert_eq!(reader.counts.scope.get(), 0);
    assert_eq!(reader.counts.issuance.get(), 0);
    assert_eq!(reader.counts.start_generation.get(), 0);
    assert_eq!(reader.counts.current_generation.get(), 0);
    assert_eq!(reader.counts.read_count(), 0);

    std::fs::remove_dir(&fixture.project).unwrap();
    std::fs::rename(moved, &fixture.project).unwrap();
}

struct ScopeSwapReader {
    inner: Reader,
    project: std::path::PathBuf,
    moved: std::path::PathBuf,
    swapped: bool,
    swap_on_marketplace_read: usize,
    marketplace_reads: usize,
}

impl ScopeSwapReader {
    fn swap_once(&mut self) {
        if self.swapped {
            return;
        }
        std::fs::rename(&self.project, &self.moved).unwrap();
        std::fs::create_dir(&self.project).unwrap();
        self.swapped = true;
    }
}

impl HostSurfaceReader for ScopeSwapReader {
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

impl TestSurfaceSource for ScopeSwapReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if self.marketplace_reads == self.swap_on_marketplace_read {
            self.swap_once();
        }
        let _ = maximum;
        self.inner.read_marketplace_raw()
    }

    fn read_cache(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_cache_raw()
    }

    fn read_registry(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        let _ = maximum;
        self.inner.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        self.inner.read_runtime_raw()
    }
}

#[test]
fn repository_scope_mutation_during_capture_and_between_capture_verify_fails_closed() {
    for boundary in ["capture", "verify"] {
        let fixture = Fixture::new(&format!("observation-scope-race-{boundary}"));
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 7);
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
        let host = fixture.host();
        let mut session = fixture.session_with_scope(
            &bundle,
            repeat,
            host.clone(),
            HostScopeAuthority::Repository {
                repository_root: fixture.project.to_string_lossy().into_owned(),
                marketplace: "local-harness-plugins".into(),
            },
        );
        session.apply_confined(&state).unwrap();
        let original_tree = fixture.tree();
        let moved = fixture.root.join(format!("project-original-{boundary}"));
        let mut reader = ScopeSwapReader {
            inner: Reader::complete(&bundle, &host, session.binding()),
            project: fixture.project.clone(),
            moved: moved.clone(),
            swapped: false,
            swap_on_marketplace_read: if boundary == "capture" { 1 } else { 3 },
            marketplace_reads: 0,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        let mutated = fixture.tree();
        assert_eq!(
            fixture.tree(),
            mutated,
            "{boundary} race wrote after failure"
        );
        assert_eq!(error.id(), HostLifecycleErrorId::HostScopeRejected);
        assert!(!error.to_string().contains(&moved.to_string_lossy()[..]));

        std::fs::remove_dir(&fixture.project).unwrap();
        std::fs::rename(&moved, &fixture.project).unwrap();
        assert_eq!(
            fixture.tree(),
            original_tree,
            "scope race changed retained state"
        );
        let mut current = Reader::complete(&bundle, &host, session.binding());
        assert_eq!(
            session.capture_and_verify(&mut current).unwrap_err().id(),
            HostLifecycleErrorId::SessionStateRejected,
            "scope rejection revived at {boundary}"
        );
        assert_eq!(fixture.tree(), original_tree);
    }
}
