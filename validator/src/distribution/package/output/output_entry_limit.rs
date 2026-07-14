const OUTPUT_ENTRY_LIMIT: usize = 2;
const OUTPUT_BYTE_LIMIT: usize = 65 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Complete mutation authority for one verified package artifact pair.
///
/// The permit is intentionally issued only inside the crate after a
/// `PackageSnapshot` has fixed every source, package, and inventory identity.
/// External callers cannot construct or alter it.
///
/// ```compile_fail,E0624
/// use ultragoal::distribution::{PackageArtifactBinding, PackageSnapshot};
/// fn forge(snapshot: &PackageSnapshot) {
///     let _ = PackageArtifactBinding::issue(snapshot);
/// }
/// ```
pub struct PackageArtifactBinding {
    context_id: String,
    candidate_id: String,
    plugin_id: String,
    version: String,
    catalog_id: String,
    accepted_inventory_sha256: String,
    source_tree_sha256: String,
    package_sha256: String,
    inventory_sha256: String,
}

impl PackageArtifactBinding {
    pub(crate) fn issue(snapshot: &PackageSnapshot) -> Result<Self, DistributionError> {
        verify_snapshot(snapshot)?;
        let source = snapshot.identity().source();
        Ok(Self {
            context_id: snapshot.context_id().into(),
            candidate_id: snapshot.candidate_id().into(),
            plugin_id: source.plugin_id().into(),
            version: source.version().into(),
            catalog_id: snapshot.catalog_id().into(),
            accepted_inventory_sha256: snapshot.accepted_inventory_sha256().into(),
            source_tree_sha256: snapshot.source_tree_sha256().into(),
            package_sha256: snapshot.package_sha256().into(),
            inventory_sha256: snapshot.inventory_sha256().into(),
        })
    }

    #[cfg(test)]
    pub(crate) fn substituted(&self, dimension: &str, value: &str) -> Self {
        let mut altered = self.clone();
        match dimension {
            "context_id" => altered.context_id = value.into(),
            "candidate_id" => altered.candidate_id = value.into(),
            "plugin_id" => altered.plugin_id = value.into(),
            "version" => altered.version = value.into(),
            "catalog_id" => altered.catalog_id = value.into(),
            "accepted_inventory_sha256" => altered.accepted_inventory_sha256 = value.into(),
            "source_tree_sha256" => altered.source_tree_sha256 = value.into(),
            "package_sha256" => altered.package_sha256 = value.into(),
            "inventory_sha256" => altered.inventory_sha256 = value.into(),
            _ => panic!("unknown package binding dimension"),
        }
        altered
    }
}

#[derive(Debug)]
pub struct PackageArtifactTransaction {
    output_tree_sha256: String,
    previous: Option<Vec<TreeObject>>,
}

impl PackageArtifactTransaction {
    pub fn output_tree_sha256(&self) -> &str {
        &self.output_tree_sha256
    }
}

pub fn publish_package_artifact(
    snapshot: &PackageSnapshot,
    binding: &PackageArtifactBinding,
    expected: &ExpectedTree,
    effects: &mut impl MaterializeEffects,
) -> Result<PackageArtifactTransaction, DistributionError> {
    verify_binding(snapshot, binding)?;
    let replacement = expected_pair(snapshot)?;
    let previous = read(effects)?;
    let previous_sha256 = previous.as_deref().map(tree_sha256).transpose()?;
    if previous
        .as_deref()
        .is_some_and(|rows| !is_complete_artifact_pair(rows))
        || !expected_matches(expected, previous_sha256.as_deref())
        || collides(previous.as_deref(), &replacement)
    {
        return Err(error(DistributionErrorId::InstallConflict));
    }
    match effects.compare_exchange_tree(previous_sha256.as_deref(), Some(&replacement)) {
        Ok(true) => {}
        Ok(false) => return Err(error(DistributionErrorId::InstallConflict)),
        Err(()) => return Err(error(DistributionErrorId::EffectFailed)),
    }
    let result = read(effects).and_then(|observed| {
        reconcile_package_artifact(snapshot, binding, observed.as_deref())?;
        Ok(())
    });
    if let Err(failure) = result {
        restore(
            effects,
            tree_sha256(&replacement)?.as_str(),
            previous.as_deref(),
        )?;
        return Err(failure);
    }
    Ok(PackageArtifactTransaction {
        output_tree_sha256: tree_sha256(&replacement)?,
        previous,
    })
}

pub fn reconcile_package_artifact(
    snapshot: &PackageSnapshot,
    binding: &PackageArtifactBinding,
    observed: Option<&[TreeObject]>,
) -> Result<(), DistributionError> {
    verify_binding(snapshot, binding)?;
    let expected = expected_pair(snapshot)?;
    let observed = observed.ok_or_else(|| error(DistributionErrorId::ArchiveMismatch))?;
    tree_sha256(observed)?;
    if observed != expected {
        return Err(error(DistributionErrorId::ArchiveMismatch));
    }
    Ok(())
}

pub fn rollback_package_artifact(
    transaction: PackageArtifactTransaction,
    effects: &mut impl MaterializeEffects,
) -> Result<(), DistributionError> {
    restore(
        effects,
        &transaction.output_tree_sha256,
        transaction.previous.as_deref(),
    )
}

pub fn recover_package_artifact(output: &ScopedTree) -> Result<bool, DistributionError> {
    output.recover_interrupted()
}

fn verify_binding(
    snapshot: &PackageSnapshot,
    binding: &PackageArtifactBinding,
) -> Result<(), DistributionError> {
    verify_snapshot(snapshot)?;
    let source = snapshot.identity().source();
    if snapshot.context_id() != binding.context_id
        || snapshot.candidate_id() != binding.candidate_id
        || source.plugin_id() != binding.plugin_id
        || source.version() != binding.version
        || snapshot.catalog_id() != binding.catalog_id
        || snapshot.accepted_inventory_sha256() != binding.accepted_inventory_sha256
        || snapshot.source_tree_sha256() != binding.source_tree_sha256
        || snapshot.package_sha256() != binding.package_sha256
        || snapshot.inventory_sha256() != binding.inventory_sha256
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}

fn verify_snapshot(snapshot: &PackageSnapshot) -> Result<(), DistributionError> {
    let source = snapshot.identity().source();
    if !digest(snapshot.context_id())
        || !digest(snapshot.candidate_id())
        || !digest(snapshot.catalog_id())
        || !digest(snapshot.accepted_inventory_sha256())
        || !digest(snapshot.source_tree_sha256())
        || !digest(snapshot.package_sha256())
        || !digest(snapshot.inventory_sha256())
        || snapshot.context_id() != source.context_id()
        || snapshot.candidate_id() != source.candidate_id()
        || snapshot.catalog_id() != source.catalog_id()
        || snapshot.accepted_inventory_sha256() != source.accepted_inventory_sha256()
        || snapshot.package_sha256() != sha256(snapshot.archive())
        || snapshot.inventory_sha256() != sha256(snapshot.inventory())
        || snapshot.identity().archive_sha256() != snapshot.package_sha256()
        || snapshot.identity().tree_sha256() != snapshot.source_tree_sha256()
    {
        return Err(error(DistributionErrorId::ProvenanceMismatch));
    }
    Ok(())
}
