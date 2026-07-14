#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostAdapterKind {
    IsolatedFilesystem,
    CodexApp,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HostCapabilityState {
    Supported,
    Unsupported,
    Absent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct HostCapabilityDeclaration {
    adapter: HostAdapterKind,
    platform: String,
    host_version: String,
    home_id: String,
    project_id: String,
    host_id: String,
    capabilities: BTreeMap<Capability, HostCapabilityState>,
    capability_sha256: String,
}

impl HostCapabilityDeclaration {
    pub fn isolated(
        home: &Path,
        project: &Path,
        host_version: &str,
        runtime_program: Option<&Path>,
    ) -> Result<Self, DistributionError> {
        let home_id = directory_id(home)?;
        let project_id = directory_id(project)?;
        let runtime = match runtime_program {
            Some(path) if executable(path)? => HostCapabilityState::Supported,
            Some(_) => HostCapabilityState::Unsupported,
            None => HostCapabilityState::Absent,
        };
        let mut capabilities = Capability::ALL
            .into_iter()
            .map(|capability| (capability, HostCapabilityState::Supported))
            .collect::<BTreeMap<_, _>>();
        capabilities.insert(Capability::PluginsUi, HostCapabilityState::Unsupported);
        capabilities.insert(Capability::Runtime, runtime);
        Self::new(
            HostAdapterKind::IsolatedFilesystem,
            current_platform(),
            host_version,
            home_id,
            project_id,
            capabilities,
        )
    }

    pub fn unavailable_codex_app(
        home: &Path,
        project: &Path,
        host_version: &str,
    ) -> Result<Self, DistributionError> {
        let mut capabilities = Capability::ALL
            .into_iter()
            .map(|capability| (capability, HostCapabilityState::Absent))
            .collect::<BTreeMap<_, _>>();
        capabilities.insert(Capability::Filesystem, HostCapabilityState::Supported);
        capabilities.insert(Capability::AppRegistry, HostCapabilityState::Unsupported);
        capabilities.insert(Capability::PluginsUi, HostCapabilityState::Unsupported);
        capabilities.insert(Capability::Discovery, HostCapabilityState::Unsupported);
        capabilities.insert(Capability::Runtime, HostCapabilityState::Unsupported);
        Self::new(
            HostAdapterKind::CodexApp,
            current_platform(),
            host_version,
            directory_id(home)?,
            directory_id(project)?,
            capabilities,
        )
    }

    fn new(
        adapter: HostAdapterKind,
        platform: &str,
        host_version: &str,
        home_id: String,
        project_id: String,
        capabilities: BTreeMap<Capability, HostCapabilityState>,
    ) -> Result<Self, DistributionError> {
        if host_version.is_empty()
            || host_version.len() > 128
            || host_version.bytes().any(|byte| byte.is_ascii_control())
            || capabilities.len() != Capability::ALL.len()
            || Capability::ALL
                .iter()
                .any(|row| !capabilities.contains_key(row))
        {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        let host_id = sha256(
            format!("{adapter:?}\0{platform}\0{host_version}\0{home_id}\0{project_id}").as_bytes(),
        );
        #[derive(Serialize)]
        struct CapabilityBinding<'a> {
            schema: &'static str,
            adapter: HostAdapterKind,
            platform: &'a str,
            host_version: &'a str,
            home_id: &'a str,
            project_id: &'a str,
            host_id: &'a str,
            capabilities: &'a BTreeMap<Capability, HostCapabilityState>,
        }
        let capability_sha256 = serde_json::to_vec(&CapabilityBinding {
            schema: "harness-ultragoal.host-capabilities.v1",
            adapter,
            platform,
            host_version,
            home_id: &home_id,
            project_id: &project_id,
            host_id: &host_id,
            capabilities: &capabilities,
        })
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
        Ok(Self {
            adapter,
            platform: platform.into(),
            host_version: host_version.into(),
            home_id,
            project_id,
            host_id,
            capabilities,
            capability_sha256,
        })
    }

    pub fn state(&self, capability: Capability) -> HostCapabilityState {
        self.capabilities[&capability]
    }
    pub fn home_id(&self) -> &str {
        &self.home_id
    }
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
    pub fn host_id(&self) -> &str {
        &self.host_id
    }
    pub fn capability_sha256(&self) -> &str {
        &self.capability_sha256
    }
    pub fn adapter(&self) -> HostAdapterKind {
        self.adapter
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct JourneyBinding {
    package: PackageIdentity,
    marketplace: String,
    home_id: String,
    project_id: String,
    host_id: String,
    capability_sha256: String,
    binding_sha256: String,
}
