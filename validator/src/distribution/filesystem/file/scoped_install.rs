impl InstallEffects for ScopedInstall {
    fn read_installed(&mut self, target: &str, maximum: usize) -> Result<Option<Vec<u8>>, ()> {
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.inspect(maximum))
            .map_err(|_| ())
    }

    fn installed_postimage(
        &mut self,
        target: &str,
        maximum: usize,
    ) -> Result<Option<InstalledPostimage>, ()> {
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.installed_postimage(maximum))
            .map_err(|_| ())
    }

    fn current_install_authority(
        &mut self,
        snapshot: &InstallSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
    ) -> Result<CurrentInstallAuthority, ()> {
        ScopedFile::new(self.root.clone(), snapshot.target())
            .and_then(|file| CurrentInstallAuthority::issue(snapshot, binding, file))
            .map_err(|_| ())
    }

    fn compare_exchange_installed(
        &mut self,
        target: &str,
        expected: &ExpectedPrior,
        replacement: Option<&[u8]>,
    ) -> Result<bool, ()> {
        let expected = match expected {
            ExpectedPrior::Absent => None,
            ExpectedPrior::ExactDigest(value) => Some(value.as_str()),
        };
        ScopedFile::new(self.root.clone(), target)
            .and_then(|row| row.apply(expected, replacement))
            .map_err(|_| ())
    }
}
