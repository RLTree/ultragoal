impl TestSurfaceSource for InstalledDriftReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        if self.marketplace_reads == 3 {
            std::fs::write(&self.installed_path, &self.replacement).unwrap();
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
fn installed_surface_is_reobserved_after_capture_before_decision() {
    let fixture = Fixture::new("post-capture-installed-drift");
    let bundle = fixture.bundle("0.0.12");
    let substitute = fixture.bundle("0.0.13");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 11);
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
    let installed_path = fixture.root.join("installed/harness-ultragoal.hugpkg");
    let original = std::fs::read(&installed_path).unwrap();
    let before_tree = fixture.tree();
    let mut reader = InstalledDriftReader {
        inner: Reader::complete(&bundle, &host, session.binding()),
        installed_path: installed_path.clone(),
        replacement: substitute.snapshot.archive().to_vec(),
        marketplace_reads: 0,
    };
    let error = session.capture_and_verify(&mut reader).unwrap_err();
    assert!(matches!(
        error.id(),
        HostLifecycleErrorId::InvalidBinding
            | HostLifecycleErrorId::IdentityMismatch
            | HostLifecycleErrorId::ObservationChanged
    ));
    assert!(
        !error
            .to_string()
            .contains(&installed_path.to_string_lossy()[..])
    );
    std::fs::write(&installed_path, original).unwrap();
    assert_eq!(fixture.tree(), before_tree);

    let mut current = Reader::complete(&bundle, &host, session.binding());
    session.capture_and_verify(&mut current).unwrap();
    assert_eq!(fixture.tree(), before_tree);
}

#[derive(Clone, Copy)]
enum GenerationMutation {
    Increment,
    Rollback,
    MutateRestore,
}

struct GenerationMutationReader {
    inner: Reader,
    generation: u64,
    runtime_reads: usize,
    trigger_runtime_read: usize,
    mutation: GenerationMutation,
}

impl HostSurfaceReader for GenerationMutationReader {
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

impl TestSurfaceSource for GenerationMutationReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn generation(&self) -> u64 {
        self.generation
    }

    fn read_marketplace(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
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
        self.runtime_reads += 1;
        let observed = self.inner.read_runtime_raw();
        if self.runtime_reads == self.trigger_runtime_read {
            match self.mutation {
                GenerationMutation::Increment => {
                    self.inner.marketplace = Some(b"AFTER_LAST_LAYER_CANARY".to_vec());
                    self.generation += 1;
                }
                GenerationMutation::Rollback => self.generation -= 1,
                GenerationMutation::MutateRestore => {
                    let original = self.inner.marketplace.take();
                    self.inner.marketplace = Some(b"MUTATE_RESTORE_GENERATION_CANARY".to_vec());
                    self.inner.marketplace = original;
                    self.generation += 1;
                }
            }
        }
        observed
    }
}
