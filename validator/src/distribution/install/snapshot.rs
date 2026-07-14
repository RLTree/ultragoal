#[derive(Clone, Serialize)]
pub struct InstallSnapshot {
    pub(super) context_id: String,
    pub(super) candidate_id: String,
    pub(super) scope: InstallScope,
    pub(super) target_id: String,
    pub(super) target: String,
    pub(super) package_sha256: String,
    pub(super) replaced_existing: bool,
    pub(super) postimage: Option<InstalledPostimage>,
    pub(super) journey_binding_sha256: Option<String>,
}

impl std::fmt::Debug for InstallSnapshot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InstallSnapshot")
            .field("context_id", &self.context_id)
            .field("candidate_id", &self.candidate_id)
            .field("scope", &self.scope)
            .field("target_id", &self.target_id)
            .field("package_sha256", &self.package_sha256)
            .field("replaced_existing", &self.replaced_existing)
            .field("has_postimage", &self.postimage.is_some())
            .field(
                "has_journey_binding",
                &self.journey_binding_sha256.is_some(),
            )
            .finish()
    }
}

impl InstallSnapshot {
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub const fn scope(&self) -> InstallScope {
        self.scope
    }
    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }
    pub fn target_id(&self) -> &str {
        &self.target_id
    }
    pub(crate) fn target(&self) -> &str {
        &self.target
    }
    pub const fn replaced_existing(&self) -> bool {
        self.replaced_existing
    }
    pub(crate) fn root_id(&self) -> Option<&str> {
        self.postimage.as_ref().map(|row| row.root_id.as_str())
    }
    pub(crate) fn postimage(&self) -> Option<&InstalledPostimage> {
        self.postimage.as_ref()
    }
    pub(crate) fn journey_binding_sha256(&self) -> Option<&str> {
        self.journey_binding_sha256.as_deref()
    }
    pub(crate) fn bind_journey(
        &mut self,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<(), DistributionError> {
        self.bindable_to(binding)?;
        self.journey_binding_sha256 = Some(binding.binding_sha256().into());
        Ok(())
    }
    fn bindable_to(
        &self,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<(), DistributionError> {
        let source = binding.package().source();
        let Some(postimage) = &self.postimage else {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        };
        if self.context_id != source.context_id()
            || self.candidate_id != source.candidate_id()
            || self.package_sha256 != binding.package().archive_sha256()
            || postimage.root_id != binding.home_id()
            || self.target != "plugins/harness-ultragoal.hugpkg"
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        Ok(())
    }
    pub(crate) fn revalidate_current(
        &self,
        binding: &crate::distribution::host_capability::JourneyBinding,
        effects: &mut impl InstallEffects,
    ) -> Result<(), DistributionError> {
        let Some(expected) = &self.postimage else {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        };
        if self.journey_binding_sha256() != Some(binding.binding_sha256()) {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let current = effects
            .installed_postimage(&self.target, INSTALL_LIMIT)
            .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?
            .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
        if &current != expected {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CurrentInstallAuthority {
    snapshot: InstallSnapshot,
    binding_sha256: String,
    file: crate::distribution::filesystem::ScopedFile,
}

impl CurrentInstallAuthority {
    pub(crate) fn issue(
        snapshot: &InstallSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
        file: crate::distribution::filesystem::ScopedFile,
    ) -> Result<Self, DistributionError> {
        snapshot.bindable_to(binding)?;
        let authority = Self {
            snapshot: snapshot.clone(),
            binding_sha256: binding.binding_sha256().into(),
            file,
        };
        authority.revalidate(binding)?;
        Ok(authority)
    }

    pub(crate) fn revalidate(
        &self,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<(), DistributionError> {
        let expected = self
            .snapshot
            .postimage
            .as_ref()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        if self.binding_sha256 != binding.binding_sha256()
            || self.snapshot.journey_binding_sha256() != Some(binding.binding_sha256())
            || self.file.root_id() != binding.home_id()
            || self.file.relative_path() != self.snapshot.target()
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let current = self
            .file
            .installed_postimage(INSTALL_LIMIT)?
            .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
        if &current != expected {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }
}

pub struct InstallTransaction {
    pub(super) snapshot: InstallSnapshot,
    pub(super) target: String,
    pub(super) previous: Option<Vec<u8>>,
}

impl std::fmt::Debug for InstallTransaction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InstallTransaction")
            .field("snapshot", &self.snapshot)
            .field("target_id", &sha256(self.target.as_bytes()))
            .field("had_previous", &self.previous.is_some())
            .finish()
    }
}

impl InstallTransaction {
    pub fn snapshot(&self) -> &InstallSnapshot {
        &self.snapshot
    }
    pub fn bind_journey(
        &mut self,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<(), DistributionError> {
        self.snapshot.bind_journey(binding)
    }

    pub(crate) fn revalidate_for_rollback(
        &self,
        effects: &mut ScopedInstall,
    ) -> Result<(), DistributionError> {
        validate_relative_path(&self.target)?;
        if self.snapshot.journey_binding_sha256.is_some() {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        if effects.scope() != Some(self.snapshot.scope) {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        if self.snapshot.target_id != sha256(self.target.as_bytes()) {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let expected = self
            .snapshot
            .postimage()
            .ok_or_else(|| error(DistributionErrorId::ProvenanceMismatch))?;
        if expected.object_sha256() != self.snapshot.package_sha256 {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let current = effects
            .installed_postimage(&self.target, INSTALL_LIMIT)
            .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?
            .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
        if !current.same_location(expected) {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        if &current != expected {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }
}
