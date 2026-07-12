use super::error::{HostLifecycleError, HostLifecycleErrorId};
use crate::distribution::{
    IdentitySurface, JourneyBinding, PackageIdentity, SurfaceIdentity, verify_bound_surface_chain,
};
use sha2::{Digest, Sha256};

pub fn verify_host_identity_chain(
    package: &PackageIdentity,
    binding: &JourneyBinding,
    marketplace_sha256: &str,
    installed_observation_sha256: &str,
    cache_observation_sha256: &str,
    discovery_observation_sha256: &str,
    runtime_observation_sha256: &str,
) -> Result<String, HostLifecycleError> {
    let rows = [
        (IdentitySurface::Marketplace, marketplace_sha256, None),
        (
            IdentitySurface::Installed,
            installed_observation_sha256,
            Some(package.tree_sha256()),
        ),
        (
            IdentitySurface::Cache,
            cache_observation_sha256,
            Some(package.tree_sha256()),
        ),
        (
            IdentitySurface::Discovery,
            discovery_observation_sha256,
            None,
        ),
        (IdentitySurface::Runtime, runtime_observation_sha256, None),
    ]
    .into_iter()
    .map(|(surface, observation, tree)| {
        SurfaceIdentity::new(
            package.clone(),
            surface,
            observation.to_owned(),
            tree.map(str::to_owned),
        )
        .and_then(|row| row.bind_journey(binding))
        .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))
    })
    .collect::<Result<Vec<_>, _>>()?;
    verify_bound_surface_chain(&rows, binding)
        .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))?;
    let bytes = serde_json::to_vec(&rows)
        .map_err(|_| HostLifecycleError::new(HostLifecycleErrorId::IdentityMismatch))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
