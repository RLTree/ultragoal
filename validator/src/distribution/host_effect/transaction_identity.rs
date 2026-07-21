use super::transaction_observation::{HostLifecycleExpectedContent, HostLifecycleObservationInput};
use super::transaction_observation_identity::{
    expected_marketplace_digest, expected_plugin_digest,
};
use crate::distribution::PackageIdentity;
use crate::plugin_product::lifecycle::{HostLifecycleExpectedObservations, LifecyclePlan};
use sha2::{Digest, Sha256};

pub(crate) fn expected_observations(
    package: &PackageIdentity,
    plan: &LifecyclePlan,
    input: &HostLifecycleObservationInput,
    content: &HostLifecycleExpectedContent,
    command_count: usize,
) -> Result<HostLifecycleExpectedObservations, &'static str> {
    let installed = plan.expected_after.installed.is_some();
    let cache = plan.expected_after.cache.is_some();
    let authority = plan.expected_after.installed.as_ref();
    Ok(HostLifecycleExpectedObservations {
        installed_sha256: expected_or_absent(installed, &content.installed),
        cache_sha256: expected_or_absent(cache, &content.cache),
        registry_sha256: match authority {
            Some(_) => expected_marketplace_digest(input)?,
            None => absent_digest(),
        },
        discovery_sha256: match authority {
            Some(authority) => expected_plugin_digest(input, authority)?,
            None => absent_digest(),
        },
        runtime_sha256: expected_or_absent(installed, &content.runtime),
        command_count,
    })
    .and_then(|expected| {
        package
            .validate()
            .map_err(|_| "host observation package authority invalid")?;
        expected
            .validate()
            .map_err(|_| "host observation expected authority invalid")?;
        Ok(expected)
    })
}

fn expected_or_absent(present: bool, digest: &str) -> String {
    if present {
        digest.to_owned()
    } else {
        absent_digest()
    }
}

pub(crate) fn absent_digest() -> String {
    format!("sha256:{:x}", Sha256::digest(b"absent"))
}
