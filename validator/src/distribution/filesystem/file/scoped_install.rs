impl InstallEffects for ScopedInstall {
    fn read_installed(
        &mut self,
        target: &str,
        maximum: usize,
    ) -> Result<Option<Vec<u8>>, crate::distribution::EffectFailure> {
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.inspect(maximum))
            .map_err(|_| crate::distribution::EffectFailure)
    }

    fn installed_postimage(
        &mut self,
        target: &str,
        maximum: usize,
    ) -> Result<Option<InstalledPostimage>, crate::distribution::EffectFailure> {
        if self
            .last_postimage
            .as_ref()
            .is_some_and(|(stored, _)| stored == target)
        {
            return Ok(self.last_postimage.take().map(|(_, row)| row));
        }
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.installed_postimage(maximum))
            .map_err(|_| crate::distribution::EffectFailure)
    }

    fn current_install_authority(
        &mut self,
        snapshot: &InstallSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<CurrentInstallAuthority, crate::distribution::EffectFailure> {
        ScopedFile::new(self.root.clone(), snapshot.target())
            .and_then(|file| CurrentInstallAuthority::issue(snapshot, binding, file))
            .map_err(|_| crate::distribution::EffectFailure)
    }

    fn compare_exchange_installed(
        &mut self,
        target: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, crate::distribution::EffectFailure> {
        self.last_postimage = None;
        let expected = match expected {
            ExpectedPrior::Absent => None,
            ExpectedPrior::ExactDigest(value) => Some(value.as_str()),
        };
        let result = ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.apply_with_postimage(expected, replacement))
            .map_err(|_| crate::distribution::EffectFailure)?;
        if result.0
            && let Some(postimage) = result.1
        {
            self.last_postimage = Some((target.into(), postimage));
        }
        Ok(result.0)
    }
}

impl ScopedInstall {
    pub(crate) fn compare_exchange_installed_postimage(
        &mut self,
        target: &str,
        expected: &InstalledPostimage,
        replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        ScopedFile::new(self.root.clone(), target)?.apply_postimage(expected, replacement)
    }
}
