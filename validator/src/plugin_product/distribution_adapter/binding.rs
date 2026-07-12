use super::error::{AdapterError, AdapterErrorId};
use crate::distribution::{PackagePlan, PackageSnapshot, verify_package};
use crate::plugin_product::lifecycle::{
    LifecycleIntent, LifecyclePlan, PackageAuthority, Version, verify,
};

#[derive(Clone)]
pub(super) struct BoundPackage {
    plan: PackagePlan,
    snapshot: PackageSnapshot,
    authority: PackageAuthority,
}

impl BoundPackage {
    pub(super) fn bind(
        plan: &PackagePlan,
        snapshot: &PackageSnapshot,
        lifecycle: &LifecyclePlan,
    ) -> Result<Self, AdapterError> {
        let verified = verify_package(plan, snapshot.archive())
            .map_err(|_| AdapterError::new(AdapterErrorId::InvalidPackageBinding))?;
        if &verified != snapshot
            || plan.context_id() != snapshot.context_id()
            || plan.candidate_id() != snapshot.candidate_id()
            || plan.catalog_id() != snapshot.catalog_id()
            || plan.accepted_inventory_sha256() != snapshot.accepted_inventory_sha256()
            || plan.source_tree_sha256() != snapshot.source_tree_sha256()
        {
            return Err(AdapterError::new(AdapterErrorId::InvalidPackageBinding));
        }
        let authority = PackageAuthority {
            version: Version::parse(plan.version())
                .map_err(|_| AdapterError::new(AdapterErrorId::InvalidPackageBinding))?,
            package_sha256: snapshot.package_sha256().to_owned(),
            inventory_sha256: snapshot.inventory_sha256().to_owned(),
            candidate_id: snapshot.candidate_id().to_owned(),
        };
        bind_lifecycle(lifecycle, &authority)?;
        Ok(Self {
            plan: plan.clone(),
            snapshot: snapshot.clone(),
            authority,
        })
    }

    pub(super) fn snapshot(&self) -> &PackageSnapshot {
        &self.snapshot
    }

    pub(super) fn authority(&self) -> &PackageAuthority {
        &self.authority
    }

    pub(super) fn verify_bytes(&self, bytes: &[u8]) -> Result<(), AdapterError> {
        let verified = verify_package(&self.plan, bytes).map_err(AdapterError::distribution)?;
        if verified != self.snapshot {
            return Err(AdapterError::new(AdapterErrorId::InvalidPackageBinding));
        }
        Ok(())
    }
}

fn bind_lifecycle(
    lifecycle: &LifecyclePlan,
    authority: &PackageAuthority,
) -> Result<(), AdapterError> {
    verify(&lifecycle.expected_after, lifecycle)
        .map_err(|_| AdapterError::new(AdapterErrorId::InvalidLifecycleBinding))?;
    let primary = match lifecycle.intent {
        LifecycleIntent::UninstallTeardown => lifecycle.before.installed.as_ref(),
        _ => lifecycle.expected_after.installed.as_ref(),
    };
    if primary != Some(authority)
        || lifecycle
            .expected_after
            .installed
            .as_ref()
            .is_some_and(|row| row != authority)
        || lifecycle
            .expected_after
            .cache
            .as_ref()
            .is_some_and(|row| row != authority)
    {
        return Err(AdapterError::new(AdapterErrorId::InvalidLifecycleBinding));
    }
    Ok(())
}
