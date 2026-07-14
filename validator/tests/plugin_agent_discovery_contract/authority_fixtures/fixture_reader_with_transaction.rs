impl HostAgentAuthorityReader for FixtureReader {
    fn with_transaction<T>(
        &mut self,
        request: &HostAgentAuthorityRequest,
        operation: impl FnOnce(
            Result<&mut dyn HostAgentAuthorityTransaction, HostAgentAuthorityTransactionError>,
        ) -> T,
    ) -> T {
        self.calls += 1;
        match self.unavailable {
            Some(error) => operation(Err(error)),
            None => {
                let mut transaction = FixtureTransaction::exact(&self.source, request);
                if let Some(configure) = self.configure.take() {
                    configure(&mut transaction);
                }
                self.transaction = Some(transaction);
                operation(Ok(self.transaction.as_mut().unwrap()))
            }
        }
    }
}

pub struct FixtureTransaction {
    pub provenance: String,
    pub project: String,
    pub candidate: String,
    pub session: String,
    pub issuance: String,
    pub nonce: String,
    pub start_generation: u64,
    pub current_generation: u64,
    pub catalogs: BTreeMap<AgentAuthorityLayer, Vec<u8>>,
    pub second_catalogs: Option<BTreeMap<AgentAuthorityLayer, Vec<u8>>>,
    pub mutate_path_on_read: Option<(usize, PathBuf, Vec<u8>)>,
    pub reads: usize,
    pub probes: usize,
    pub forge_effect: bool,
}

impl FixtureTransaction {
    pub fn exact(source: &SourceAgentCatalog, request: &HostAgentAuthorityRequest) -> Self {
        let catalogs = AgentAuthorityLayer::ALL
            .into_iter()
            .map(|layer| (layer, catalog_bytes(source, request, layer, Vec::new())))
            .collect();
        Self {
            provenance: request.provenance_sha256().to_owned(),
            project: request.project_root_sha256().to_owned(),
            candidate: request.candidate_id().to_owned(),
            session: request.session_id().to_owned(),
            issuance: request.session_issuance_sha256().to_owned(),
            nonce: request.observation_nonce_sha256().to_owned(),
            start_generation: 7,
            current_generation: 7,
            catalogs,
            second_catalogs: None,
            mutate_path_on_read: None,
            reads: 0,
            probes: 0,
            forge_effect: false,
        }
    }

    pub fn mutate_catalog(&mut self, layer: AgentAuthorityLayer, mutate: impl FnOnce(&mut Value)) {
        let mut value: Value = serde_json::from_slice(&self.catalogs[&layer]).unwrap();
        mutate(&mut value);
        self.catalogs
            .insert(layer, serde_json::to_vec(&value).unwrap());
    }

    pub fn add_global_agent(&mut self, name: &str, sandbox: Option<&str>) {
        let descriptor = match sandbox {
            Some(sandbox) => descriptor(name, sandbox),
            None => format!(
                "name = \"{name}\"\ndescription = \"legacy\"\ndeveloper_instructions = \"legacy authority\"\n"
            ),
        };
        self.add_global_agent_bytes(name, &format!("custom-agents/{name}.toml"), descriptor);
    }

    pub fn add_global_agent_bytes(
        &mut self,
        name: &str,
        manifest_path: &str,
        descriptor: impl Into<String>,
    ) {
        let descriptor = descriptor.into();
        let name = name.to_owned();
        let manifest_path = manifest_path.to_owned();
        self.mutate_catalog(AgentAuthorityLayer::Global, |value| {
            value["agents"].as_array_mut().unwrap().push(json!({
                "name": name,
                "manifest_path": manifest_path,
                "descriptor_sha256": digest(descriptor.as_bytes()),
                "descriptor_toml": descriptor,
                "file_kind": "regular",
                "link_count": 1
            }));
        });
    }

    pub fn remove_global_agent(&mut self, name: &str) {
        self.mutate_catalog(AgentAuthorityLayer::Global, |value| {
            value["agents"]
                .as_array_mut()
                .unwrap()
                .retain(|row| row["name"].as_str() != Some(name));
        });
    }
}

impl HostAgentAuthorityTransaction for FixtureTransaction {
    fn provenance_sha256(&self) -> &str {
        &self.provenance
    }
    fn project_root_sha256(&self) -> &str {
        &self.project
    }
    fn candidate_id(&self) -> &str {
        &self.candidate
    }
    fn session_id(&self) -> &str {
        &self.session
    }
    fn session_issuance_sha256(&self) -> &str {
        &self.issuance
    }
    fn observation_nonce_sha256(&self) -> &str {
        &self.nonce
    }
    fn start_generation(&self) -> u64 {
        self.start_generation
    }
    fn current_generation(&self) -> Result<u64, ()> {
        Ok(self.current_generation)
    }
    fn read_catalog(
        &mut self,
        layer: AgentAuthorityLayer,
        _maximum: usize,
    ) -> Result<Option<Vec<u8>>, ()> {
        let round = self.reads / AgentAuthorityLayer::ALL.len();
        self.reads += 1;
        if self
            .mutate_path_on_read
            .as_ref()
            .is_some_and(|(at, _, _)| *at == self.reads)
        {
            let (_, path, bytes) = self.mutate_path_on_read.take().unwrap();
            fs::write(path, bytes).unwrap();
        }
        let catalogs = if round > 0 {
            self.second_catalogs.as_ref().unwrap_or(&self.catalogs)
        } else {
            &self.catalogs
        };
        Ok(catalogs.get(&layer).cloned())
    }
    fn enforce_read_only(
        &mut self,
        request: &ReadOnlyEffectRequest,
    ) -> Result<ReadOnlyEffectEnforcement, ()> {
        self.probes += 1;
        Ok(if self.forge_effect {
            ReadOnlyEffectEnforcement::forged_write_capable(request)
        } else {
            ReadOnlyEffectEnforcement::denied(request)
        })
    }
}
