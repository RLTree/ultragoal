use super::binding::BoundPackage;
use super::error::{AdapterError, AdapterErrorId};
use crate::distribution::{ConfinedRoot, ScopedFile};
use crate::plugin_product::lifecycle::{LifecycleState, PackageAuthority};
use sha2::{Digest, Sha256};

pub(super) const INSTALLED_PATH: &str = "installed/harness-ultragoal.hugpkg";
pub(super) const CACHE_PATH: &str = "cache/harness-ultragoal.hugpkg";
const SURFACE_LIMIT: usize = 65 * 1024 * 1024;

pub(super) fn observe(
    root: &ConfinedRoot,
    logical: &LifecycleState,
    before: &LifecycleState,
    expected_after: &LifecycleState,
    package: &BoundPackage,
) -> Result<LifecycleState, AdapterError> {
    let installed = observe_surface(root, INSTALLED_PATH, before, expected_after, package)?;
    let cache = observe_surface(root, CACHE_PATH, before, expected_after, package)?;
    Ok(LifecycleState {
        installed,
        cache,
        generation: logical.generation,
        recovery_required: logical.recovery_required,
    })
}

pub(super) fn verify_surface(
    root: &ConfinedRoot,
    path: &str,
    expected: Option<&PackageAuthority>,
    package: &BoundPackage,
) -> Result<(), AdapterError> {
    let bytes = ScopedFile::new(root.clone(), path)
        .and_then(|file| file.inspect(SURFACE_LIMIT))
        .map_err(AdapterError::distribution)?;
    match (bytes, expected) {
        (None, None) => Ok(()),
        (Some(bytes), Some(expected)) if digest(&bytes) == expected.package_sha256 => {
            if expected == package.authority() {
                package.verify_bytes(&bytes)?;
            }
            Ok(())
        }
        _ => Err(AdapterError::new(AdapterErrorId::PhysicalStateMismatch)),
    }
}

fn observe_surface(
    root: &ConfinedRoot,
    path: &str,
    before: &LifecycleState,
    expected_after: &LifecycleState,
    package: &BoundPackage,
) -> Result<Option<PackageAuthority>, AdapterError> {
    let bytes = ScopedFile::new(root.clone(), path)
        .and_then(|file| file.inspect(SURFACE_LIMIT))
        .map_err(AdapterError::distribution)?;
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let digest = digest(&bytes);
    let mut matches = authorities(before, expected_after)
        .into_iter()
        .filter(|row| row.package_sha256 == digest)
        .collect::<Vec<_>>();
    matches.dedup();
    if matches.len() != 1 {
        return Err(AdapterError::new(AdapterErrorId::PhysicalStateMismatch));
    }
    if &matches[0] == package.authority() {
        package.verify_bytes(&bytes)?;
    }
    Ok(matches.pop())
}

fn authorities(before: &LifecycleState, expected_after: &LifecycleState) -> Vec<PackageAuthority> {
    let mut rows = [
        before.installed.as_ref(),
        before.cache.as_ref(),
        expected_after.installed.as_ref(),
        expected_after.cache.as_ref(),
    ]
    .into_iter()
    .flatten()
    .cloned()
    .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (
            &left.package_sha256,
            &left.inventory_sha256,
            &left.candidate_id,
        )
            .cmp(&(
                &right.package_sha256,
                &right.inventory_sha256,
                &right.candidate_id,
            ))
    });
    rows.dedup();
    rows
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
