use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::model::{Capability, PackageIdentity};
use crate::distribution::reader::sha256;
use crate::distribution::spec::digest;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

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

impl JourneyBinding {
    pub fn new(
        package: PackageIdentity,
        host: &HostCapabilityDeclaration,
        marketplace: impl Into<String>,
    ) -> Result<Self, DistributionError> {
        package.validate()?;
        let marketplace = marketplace.into();
        if !digest(host.home_id()) || !digest(host.project_id()) || !digest(host.host_id()) {
            return Err(error(DistributionErrorId::InvalidSpec));
        }
        validate_marketplace_name(&marketplace)?;
        #[derive(Serialize)]
        struct Raw<'a> {
            schema: &'static str,
            package: &'a PackageIdentity,
            marketplace: &'a str,
            home_id: &'a str,
            project_id: &'a str,
            host_id: &'a str,
            capability_sha256: &'a str,
        }
        let binding_sha256 = serde_json::to_vec(&Raw {
            schema: "harness-ultragoal.distribution-journey-binding.v1",
            package: &package,
            marketplace: &marketplace,
            home_id: host.home_id(),
            project_id: host.project_id(),
            host_id: host.host_id(),
            capability_sha256: host.capability_sha256(),
        })
        .map(|bytes| sha256(&bytes))
        .map_err(|_| error(DistributionErrorId::InvalidSpec))?;
        Ok(Self {
            package,
            marketplace,
            home_id: host.home_id().into(),
            project_id: host.project_id().into(),
            host_id: host.host_id().into(),
            capability_sha256: host.capability_sha256().into(),
            binding_sha256,
        })
    }

    pub fn package(&self) -> &PackageIdentity {
        &self.package
    }
    pub fn marketplace(&self) -> &str {
        &self.marketplace
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
    pub fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }
}

fn validate_marketplace_name(value: &str) -> Result<(), DistributionError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(error(DistributionErrorId::InvalidSpec));
    }
    Ok(())
}

fn directory_id(path: &Path) -> Result<String, DistributionError> {
    let canonical = path
        .canonicalize()
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    let metadata = std::fs::symlink_metadata(&canonical)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(directory_identity(&canonical, &metadata))
}

#[cfg(unix)]
fn directory_identity(path: &Path, metadata: &std::fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    sha256(format!("{}\0{}\0{}", path.display(), metadata.dev(), metadata.ino()).as_bytes())
}

#[cfg(not(unix))]
fn directory_identity(path: &Path, metadata: &std::fs::Metadata) -> String {
    sha256(format!("{}\0{}", path.display(), metadata.len()).as_bytes())
}

fn executable(path: &Path) -> Result<bool, DistributionError> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
    Ok(path.is_absolute() && metadata.is_file() && !metadata.file_type().is_symlink())
}

fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "other"
    }
}
