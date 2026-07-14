#[test]
fn final_generation_revalidation_catches_after_last_layer_and_restore_attacks_without_close() {
    let fixture = Fixture::new("transaction-generation-close-races");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 14);
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

    for (label, trigger_runtime_read, generation, mutation) in [
        (
            "after-first-frame-last-layer",
            2,
            7,
            GenerationMutation::Increment,
        ),
        ("after-fourth-runtime", 4, 7, GenerationMutation::Increment),
        ("generation-rollback", 4, 7, GenerationMutation::Rollback),
        (
            "generation-mutate-restore",
            4,
            7,
            GenerationMutation::MutateRestore,
        ),
    ] {
        let mut reader = GenerationMutationReader {
            inner: Reader::complete(&bundle, &host, session.binding()),
            generation,
            runtime_reads: 0,
            trigger_runtime_read,
            mutation,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(
            error.id(),
            HostLifecycleErrorId::ObservationChanged,
            "{label}"
        );
        assert!(!error.to_string().contains("CANARY"), "{label}");
        assert_eq!(fixture.tree(), before, "{label} wrote");
    }

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
    assert_eq!(fixture.tree(), before);
}

enum TransactionMismatch {
    ForgedScope,
    SiblingIssuance(String),
    StaleGeneration,
}

struct MismatchedTransactionReader {
    inner: Reader,
    mismatch: TransactionMismatch,
}

impl TestSurfaceSource for MismatchedTransactionReader {
    fn provenance_sha256(&self) -> &str {
        &self.inner.provenance_sha256
    }

    fn generation(&self) -> u64 {
        self.inner.generation
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
        self.inner.read_runtime_raw()
    }
}

impl HostSurfaceReader for MismatchedTransactionReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        let mut host_scope_sha256 = request.host_scope_sha256().to_owned();
        let mut session_issuance_sha256 = request.session_issuance_sha256().to_owned();
        let mut start_generation = self.inner.generation;
        match &self.mismatch {
            TransactionMismatch::ForgedScope => {
                host_scope_sha256 =
                    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                        .into();
            }
            TransactionMismatch::SiblingIssuance(sibling) => {
                session_issuance_sha256 = sibling.clone();
            }
            TransactionMismatch::StaleGeneration => start_generation += 1,
        }
        let mut transaction = TestSurfaceTransaction {
            source: self,
            host_scope_sha256,
            session_issuance_sha256,
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

#[test]
fn forged_stale_and_sibling_transactions_cannot_bind_or_close_the_session() {
    let fixture = Fixture::new("transaction-binding-attacks");
    let bundle = fixture.bundle("0.0.12");
    fixture.seed(Some(&bundle), Some(&bundle));
    let state = installed(&bundle.authority, 15);
    let repeat = || {
        lifecycle(
            &state,
            request(
                LifecycleIntent::RepeatUse,
                Some(bundle.authority.clone()),
                None,
                state.installed.as_ref(),
                false,
                false,
            ),
        )
    };
    let (mut session, host) = fixture.session(&bundle, repeat());
    let (sibling, _) = fixture.session(&bundle, repeat());
    session.apply_confined(&state).unwrap();
    let before = fixture.tree();

    for mismatch in [
        TransactionMismatch::ForgedScope,
        TransactionMismatch::SiblingIssuance(sibling.session_issuance_sha256().to_owned()),
        TransactionMismatch::StaleGeneration,
    ] {
        let mut reader = MismatchedTransactionReader {
            inner: Reader::complete(&bundle, &host, session.binding()),
            mismatch,
        };
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ObservationChanged);
        assert!(!error.to_string().contains("dddd"));
        assert_eq!(fixture.tree(), before);
    }

    let mut current = Reader::complete(&bundle, &host, session.binding());
    assert!(session.capture_and_verify(&mut current).is_ok());
    assert_eq!(fixture.tree(), before);
}

struct TransactionFailureReader {
    error: HostSurfaceTransactionError,
}
