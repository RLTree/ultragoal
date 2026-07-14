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
