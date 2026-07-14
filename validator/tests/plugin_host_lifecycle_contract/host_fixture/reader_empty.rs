impl Reader {
    pub fn empty(binding: &JourneyBinding) -> Self {
        Self {
            provenance_sha256: binding.binding_sha256().to_owned(),
            ..Self::default()
        }
    }

    pub fn complete(
        bundle: &Bundle,
        host: &HostCapabilityDeclaration,
        binding: &JourneyBinding,
    ) -> Self {
        Self {
            provenance_sha256: binding.binding_sha256().to_owned(),
            marketplace: Some(marketplace_plan(bundle).replacement().to_vec()),
            cache: Some(cache_document(bundle, host)),
            registry: Some(registry_document(binding, true, true).unwrap()),
            ui: None,
            runtime: Some(runtime_observation(binding, host)),
            mutate_marketplace_after_first: false,
            marketplace_reads: 0,
            transaction_supported: true,
            generation: 0,
        }
    }

    pub fn teardown(bundle: &Bundle, binding: &JourneyBinding) -> Self {
        Self {
            provenance_sha256: binding.binding_sha256().to_owned(),
            marketplace: Some(marketplace_plan(bundle).replacement().to_vec()),
            ..Self::default()
        }
    }

    pub fn read_marketplace_raw(&mut self) -> Result<Option<Vec<u8>>, ()> {
        self.marketplace_reads += 1;
        let mut value = self.marketplace.clone();
        if self.mutate_marketplace_after_first && self.marketplace_reads > 1 {
            value = Some(b"changed".to_vec());
        }
        Ok(value)
    }

    pub fn read_cache_raw(&self) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.cache.clone())
    }

    pub fn read_registry_raw(&self) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.registry.clone())
    }

    pub fn read_plugins_ui_raw(&self) -> Result<Option<Vec<u8>>, ()> {
        Ok(self.ui.clone())
    }

    pub fn read_runtime_raw(&self) -> Result<Option<RuntimeObservation>, ()> {
        Ok(self.runtime.clone())
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self {
            provenance_sha256:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            marketplace: None,
            cache: None,
            registry: None,
            ui: None,
            runtime: None,
            mutate_marketplace_after_first: false,
            marketplace_reads: 0,
            transaction_supported: true,
            generation: 0,
        }
    }
}

impl HostSurfaceReader for Reader {
    fn with_transaction<T>(
        &mut self,
        request: &HostObservationTransactionRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostSurfaceTransaction, HostSurfaceTransactionError>,
        ) -> T,
    ) -> T {
        if !self.transaction_supported {
            return operation(Err(HostSurfaceTransactionError::Unsupported));
        }
        let start_generation = self.generation;
        let mut transaction = ReaderTransaction {
            reader: self,
            host_scope_sha256: request.host_scope_sha256().to_owned(),
            session_issuance_sha256: request.session_issuance_sha256().to_owned(),
            start_generation,
        };
        operation(Ok(&mut transaction))
    }
}

pub struct ReaderTransaction<'a> {
    reader: &'a mut Reader,
    host_scope_sha256: String,
    session_issuance_sha256: String,
    start_generation: u64,
}

impl HostSurfaceTransaction for ReaderTransaction<'_> {
    fn provenance_sha256(&self) -> &str {
        &self.reader.provenance_sha256
    }

    fn host_scope_sha256(&self) -> &str {
        &self.host_scope_sha256
    }

    fn session_issuance_sha256(&self) -> &str {
        &self.session_issuance_sha256
    }

    fn start_generation(&self) -> u64 {
        self.start_generation
    }

    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.reader.generation)
    }

    fn read_marketplace(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_marketplace_raw()
    }

    fn read_cache(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_cache_raw()
    }

    fn read_registry(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_registry_raw()
    }

    fn read_plugins_ui(&mut self, _: usize) -> Result<Option<Vec<u8>>, ()> {
        self.reader.read_plugins_ui_raw()
    }

    fn read_runtime(&mut self) -> Result<Option<RuntimeObservation>, ()> {
        self.reader.read_runtime_raw()
    }
}

pub fn marketplace_plan(bundle: &Bundle) -> MarketplacePlan {
    marketplace_plan_named(bundle, "local-harness-plugins")
}

pub fn marketplace_plan_named(bundle: &Bundle, marketplace: &str) -> MarketplacePlan {
    plan_codex_marketplace(
        None,
        None,
        marketplace,
        "Local Harness Plugins",
        CodexPlugin::harness_ultragoal(),
        bundle.snapshot.identity().clone(),
    )
    .unwrap()
}

pub fn cache_document(bundle: &Bundle, host: &HostCapabilityDeclaration) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema":"harness-ultragoal.codex-cache-observation.v1",
        "context_id":CONTEXT, "candidate_id":CANDIDATE,
        "cache_root_id":host.home_id(),
        "entries":[{
            "marketplace":"local-harness-plugins", "plugin_id":"harness-ultragoal",
            "version":bundle.plan.version(),
            "package_tree_sha256":bundle.snapshot.identity().tree_sha256()
        }]
    }))
    .unwrap()
}
