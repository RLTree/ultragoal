const OUTPUT_LIMIT: usize = 1024 * 1024;
const EXECUTABLE_LIMIT: usize = 256 * 1024 * 1024;
const SUPPORTED_INSTALL_TARGET: &str = "plugins/harness-ultragoal.hugpkg";
const HELP_ARGUMENTS: [&str; 2] = ["--json", "--help"];

#[derive(Clone, Debug)]
pub struct RuntimeProbePlan {
    binding: JourneyBinding,
    install: Option<CurrentInstallAuthority>,
    executable: PinnedRuntimeExecutable,
    executable_sha256: String,
    timeout: Duration,
}

pub struct InstalledPackageRuntimeProbeRequest<'a, Effects> {
    pub binding: JourneyBinding,
    pub host: &'a HostCapabilityDeclaration,
    pub install: &'a InstallSnapshot,
    pub effects: &'a mut Effects,
    pub package: &'a PackageSnapshot,
    pub program: &'a Path,
    pub timeout: Duration,
}

/// Publishes the sole executable authenticated by `package` at the supported
/// host path. The postimage is read back before the path is returned, so a
/// package entry and a host executable cannot be silently conflated.
pub fn publish_installed_runtime_probe(
    package: &PackageSnapshot,
    executable: &crate::distribution::filesystem::ScopedFile,
) -> Result<(), DistributionError> {
    if executable.relative_path() != SUPPORTED_RUNTIME_PROGRAM {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    let entries = package
        .entries()
        .iter()
        .filter(|row| row.role() == PackageRole::Executable)
        .collect::<Vec<_>>();
    let Some(entry) = entries.first() else {
        return Err(error(DistributionErrorId::CapabilityMismatch));
    };
    if entries.len() != 1 || entry.path() != PACKAGE_RUNTIME_ENTRY {
        return Err(error(DistributionErrorId::CapabilityMismatch));
    }
    let current = executable.inspect(EXECUTABLE_LIMIT)?;
    let expected = current.as_deref().map(sha256);
    let current_mode = executable
        .installed_postimage(EXECUTABLE_LIMIT)?
        .map(|postimage| postimage.mode());
    if (current.as_deref() != Some(entry.bytes()) || current_mode != Some(0o755))
        && !executable.apply_executable(expected.as_deref(), Some(entry.bytes()))?
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    let observed = executable
        .inspect(EXECUTABLE_LIMIT)?
        .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
    if observed != entry.bytes()
        || sha256(&observed) != entry.sha256()
        || executable
            .installed_postimage(EXECUTABLE_LIMIT)?
            .is_none_or(|postimage| postimage.mode() != 0o755)
    {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    Ok(())
}

impl RuntimeProbePlan {
    pub fn from_installed_package<Effects: crate::distribution::install::InstallEffects>(
        request: InstalledPackageRuntimeProbeRequest<'_, Effects>,
    ) -> Result<Self, DistributionError> {
        let InstalledPackageRuntimeProbeRequest {
            binding,
            host,
            install,
            effects,
            package,
            program,
            timeout,
        } = request;
        host.ensure_binding(&binding)?;
        install.revalidate_current(&binding, effects)?;
        let executable = PinnedRuntimeExecutable::open(program)?;
        let executable_sha256 = executable.sha256().to_owned();
        let runtime_entry = package
            .entries()
            .iter()
            .filter(|row| row.role() == PackageRole::Executable)
            .collect::<Vec<_>>();
        let Some(entry) = runtime_entry.first() else {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        };
        if host.state(Capability::Runtime) != HostCapabilityState::Supported
            || runtime_entry.len() != 1
            || entry.path() != PACKAGE_RUNTIME_ENTRY
            || entry.sha256() != executable_sha256
            || package.identity() != binding.package()
            || install.context_id() != binding.package().source().context_id()
            || install.candidate_id() != binding.package().source().candidate_id()
            || install.package_sha256() != binding.package().archive_sha256()
            || install.target_id() != sha256(SUPPORTED_INSTALL_TARGET.as_bytes())
            || !program.is_absolute()
            || !host.matches_runtime_program(program)?
            || timeout.is_zero()
            || timeout > Duration::from_secs(30)
        {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }
        let install = effects
            .current_install_authority(install, &binding)
            .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?;
        Ok(Self {
            binding,
            install: Some(install),
            executable,
            executable_sha256,
            timeout,
        })
    }

    pub(crate) fn from_verified_host_runtime(
        binding: JourneyBinding,
        host: &HostCapabilityDeclaration,
        package: &PackageSnapshot,
        program: &Path,
        timeout: Duration,
    ) -> Result<Self, DistributionError> {
        host.ensure_binding(&binding)?;
        let executable = PinnedRuntimeExecutable::open(program)?;
        let executable_sha256 = executable.sha256().to_owned();
        let entries = package
            .entries()
            .iter()
            .filter(|row| row.role() == PackageRole::Executable)
            .collect::<Vec<_>>();
        let Some(entry) = entries.first() else {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        };
        if host.state(Capability::Runtime) != HostCapabilityState::Supported
            || entries.len() != 1
            || entry.path() != PACKAGE_RUNTIME_ENTRY
            || entry.sha256() != executable_sha256
            || package.identity() != binding.package()
            || !program.is_absolute()
            || !host.matches_runtime_program(program)?
            || timeout.is_zero()
            || timeout > Duration::from_secs(30)
        {
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }
        Ok(Self {
            binding,
            install: None,
            executable,
            executable_sha256,
            timeout,
        })
    }

    pub fn execute_bound(self) -> Result<(RuntimeObservation, SurfaceIdentity), DistributionError> {
        let observation = execute_runtime_probe(&self)?;
        let output_sha256 = observation
            .output_sha256()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        let surface = SurfaceIdentity::new(
            self.binding.package().clone(),
            IdentitySurface::Runtime,
            output_sha256.into(),
            None,
        )?
        .bind_journey(&self.binding)?;
        Ok((observation, surface))
    }
}
