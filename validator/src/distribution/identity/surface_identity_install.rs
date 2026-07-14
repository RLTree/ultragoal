impl SurfaceIdentity {
    pub fn from_verified_install(
        snapshot: &crate::distribution::install::InstallSnapshot,
        binding: &crate::distribution::host_capability::JourneyBinding,
        effects: &mut impl crate::distribution::install::InstallEffects,
    ) -> Result<Self, DistributionError> {
        let package = binding.package();
        let source = package.source();
        if snapshot.context_id() != source.context_id()
            || snapshot.candidate_id() != source.candidate_id()
            || snapshot.package_sha256() != package.archive_sha256()
            || snapshot.target_id()
                != crate::distribution::reader::sha256(b"plugins/harness-ultragoal.hugpkg")
            || snapshot.root_id() != Some(binding.home_id())
            || snapshot.journey_binding_sha256() != Some(binding.binding_sha256())
        {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        snapshot.revalidate_current(binding, effects)?;
        let serialized = serde_json::to_vec(snapshot)
            .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?;
        Self::new(
            package.clone(),
            IdentitySurface::Installed,
            crate::distribution::reader::sha256(&serialized),
            Some(package.tree_sha256().into()),
        )?
        .bind_journey(binding)
    }
}
