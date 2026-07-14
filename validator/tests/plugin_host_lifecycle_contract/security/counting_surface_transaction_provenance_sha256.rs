impl HostSurfaceTransaction for CountingSurfaceTransaction<'_> {
    fn provenance_sha256(&self) -> &str {
        AdapterInvocationCounts::increment(&self.counts.provenance);
        &self.reader.provenance_sha256
    }

    fn host_scope_sha256(&self) -> &str {
        AdapterInvocationCounts::increment(&self.counts.scope);
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        AdapterInvocationCounts::increment(&self.counts.issuance);
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        AdapterInvocationCounts::increment(&self.counts.start_generation);
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        AdapterInvocationCounts::increment(&self.counts.current_generation);
        Ok(self.reader.generation)
    }

    fn read_marketplace(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.marketplace_reads);
        self.reader.read_marketplace_raw()
    }

    fn read_cache(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.cache_reads);
        self.reader.read_cache_raw()
    }

    fn read_registry(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.registry_reads);
        self.reader.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, _maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        AdapterInvocationCounts::increment(&self.counts.ui_reads);
        self.reader.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<crate::distribution::RuntimeObservation>, ()> {
        AdapterInvocationCounts::increment(&self.counts.runtime_reads);
        self.reader.read_runtime_raw()
    }
}

#[test]
fn invalid_local_preflight_states_make_exactly_zero_adapter_calls() {
    {
        let fixture = Fixture::new("preflight-not-applied");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 18);
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
        let (mut session, _) = fixture.session(&bundle, repeat);
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ExternalEffectNotEligible);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-closed");
        let bundle = fixture.bundle("0.0.12");
        fixture.seed(Some(&bundle), Some(&bundle));
        let state = installed(&bundle.authority, 19);
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
        let mut current = Reader::complete(&bundle, &host, session.binding());
        session.capture_and_verify(&mut current).unwrap();
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::SessionStateRejected);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-effect-not-ready");
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let fresh = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let (mut session, _) = fixture.session(&bundle, fresh);
        session.apply_confined(&empty).unwrap();
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::ExternalEffectNotEligible);
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }

    {
        let fixture = Fixture::new("preflight-unsupported-capability");
        let bundle = fixture.bundle("0.0.12");
        let empty = LifecycleState::default();
        let fresh = lifecycle(
            &empty,
            request(
                LifecycleIntent::FreshInstall,
                Some(bundle.authority.clone()),
                None,
                None,
                true,
                false,
            ),
        );
        let unavailable = crate::distribution::HostCapabilityDeclaration::unavailable_codex_app(
            &fixture.root,
            &fixture.project,
            "UNSUPPORTED_PREFLIGHT_CANARY",
        )
        .unwrap();
        let mut session = fixture.session_with_scope(
            &bundle,
            fresh,
            unavailable,
            HostScopeAuthority::Personal {
                marketplace: "local-harness-plugins".into(),
            },
        );
        let before = fixture.tree();
        let mut reader = CountingUnsupportedReader::default();
        let error = session.capture_and_verify(&mut reader).unwrap_err();
        assert_eq!(error.id(), HostLifecycleErrorId::HostCapabilityRejected);
        assert!(!error.to_string().contains("CANARY"));
        assert!(reader.counts.is_zero());
        assert_eq!(fixture.tree(), before);
    }
}
