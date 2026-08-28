use super::PersonalMarketplaceInstallPlan;
use crate::context::LiveContext;
use crate::distribution::{
    ConfinedRoot, MaterializeEffects, MaterializeTransaction, PERSONAL_MARKETPLACE_SOURCE_RELATIVE,
    PackageIdentity, PersonalMarketplaceAuthorityMatch, PersonalMarketplaceSourceObservation,
    PersonalMarketplaceUpdateAuthority, PersonalMarketplaceUpdateEffects,
    PersonalMarketplaceUpdateObservation, ProductionPackageArtifact, ScopedTree, TreeObject,
    tree_sha256,
};
use crate::inventory::AuthorityCatalog;
use crate::plugin_product::agent_discovery::{
    capture_installed_source_authority, parse_unpinned_host_plugin_registry_observation,
};
use crate::plugin_product::lifecycle::PriorInstalledAuthority;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(super) struct LivePersonalMarketplaceUpdateEffects<'a> {
    source_context: &'a LiveContext,
    observation_context: &'a LiveContext,
    home: PathBuf,
    record: PersonalMarketplaceInstallPlan,
    prior: PriorInstalledAuthority,
    artifact: ProductionPackageArtifact,
    catalog: AuthorityCatalog,
    package: PackageIdentity,
    home_root: ConfinedRoot,
    source_tree: ScopedTree,
    lease: PersonalMarketplaceLease,
    executable: Option<crate::distribution::host_effect::SelectedCodexExecutable>,
    cancellation: crate::distribution::host_effect::HostEffectCancellation,
    materialization: Option<MaterializeTransaction>,
    prior_tree: Option<Vec<TreeObject>>,
}

impl<'a> LivePersonalMarketplaceUpdateEffects<'a> {
    pub(super) fn new(
        source_context: &'a LiveContext,
        observation_context: &'a LiveContext,
        home: &Path,
        record: PersonalMarketplaceInstallPlan,
        prior: PriorInstalledAuthority,
        artifact: ProductionPackageArtifact,
        catalog: AuthorityCatalog,
    ) -> Result<Self, &'static str> {
        source_context
            .revalidate()
            .map_err(|_| "candidate source changed before live adapter binding")?;
        observation_context
            .revalidate()
            .map_err(|_| "host observation context changed before live adapter binding")?;
        let requested_home = home;
        let home = requested_home
            .canonicalize()
            .map_err(|_| "personal home authority is unavailable")?;
        if home != requested_home || !home.is_absolute() {
            return Err("personal home authority changed");
        }
        let lease = PersonalMarketplaceLease::acquire(&home)?;
        let home_id = directory_id(&home)?;
        let root = ConfinedRoot::open_personal_home(&home, &home_id)
            .map_err(|_| "personal home confinement failed")?;
        let source_tree = ScopedTree::new(root.clone(), PERSONAL_MARKETPLACE_SOURCE_RELATIVE)
            .map_err(|_| "personal marketplace source confinement failed")?;
        let prior_cache_relative = format!(
            ".codex/plugins/cache/{}/harness-ultragoal/{}",
            super::MARKETPLACE,
            record.before.plugin_version,
        );
        let prior_tree = ScopedTree::new(root.clone(), &prior_cache_relative)
            .ok()
            .and_then(|tree| tree.inspect(4096, 65 * 1024 * 1024).ok().flatten())
            .filter(|rows| {
                tree_sha256(rows)
                    .is_ok_and(|sha256| sha256 == record.before.marketplace_source_tree_sha256)
            });
        let executable = crate::distribution::resolve_codex_executable()
            .map_err(|_| "pinned Codex executable unavailable")?;
        let package = artifact.snapshot().identity().clone();
        Ok(Self {
            source_context,
            observation_context,
            home,
            record,
            prior,
            artifact,
            catalog,
            package,
            home_root: root,
            source_tree,
            lease,
            executable: Some(executable),
            cancellation: crate::distribution::host_effect::HostEffectCancellation::default(),
            materialization: None,
            prior_tree,
        })
    }

    pub(super) fn finish(mut self) -> Result<(), &'static str> {
        self.require_current()?;
        self.executable
            .take()
            .ok_or("pinned Codex executable was already finalized")?
            .finalize_personal()
    }

    fn require_current(&self) -> Result<(), &'static str> {
        self.lease.revalidate()?;
        self.source_context
            .revalidate()
            .map_err(|_| "candidate source changed during live personal update")?;
        self.observation_context
            .revalidate()
            .map_err(|_| "host observation context changed during live personal update")
    }

    fn executable(
        &self,
    ) -> Result<&crate::distribution::host_effect::SelectedCodexExecutable, &'static str> {
        self.executable
            .as_ref()
            .ok_or("pinned Codex executable is unavailable")
    }

    fn require_authority(
        &self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str> {
        if authority.plan_sha256() != self.record.plan_sha256
            || authority.prior() != &self.prior
            || authority.prior_tree_sha256() != self.record.before.marketplace_source_tree_sha256
            || authority.target_package_sha256() != self.record.target.archive_sha256
            || authority.target_tree_sha256() != self.record.target.source_tree_sha256
            || authority.target_package_sha256() != self.package.archive_sha256()
            || authority.target_tree_sha256() != self.package.tree_sha256()
        {
            return Err("personal marketplace update authority changed");
        }
        self.require_current()
    }

    fn capture_observation(&self) -> Result<PersonalMarketplaceUpdateObservation, &'static str> {
        self.require_current()?;
        let (selected, plugin_json, marketplace_json) =
            self.executable()?.observe_personal_plugin(
                &self.home,
                &self.record.plan_sha256,
                &self.record.before.selected_codex_identity_sha256,
                &self.cancellation,
            )?;
        let registry = parse_unpinned_host_plugin_registry_observation(
            &plugin_json,
            &marketplace_json,
            &selected,
            &self.record.before.selected_codex_identity_sha256,
        )
        .map_err(|_| "pinned Codex registry observation is invalid")?;
        let installed_root = self.home.join(PERSONAL_MARKETPLACE_SOURCE_RELATIVE);
        let cache_root = self.home.join(format!(
            ".codex/plugins/cache/local-harness-plugins/harness-ultragoal/{}",
            registry.plugin_version()
        ));
        let cache_relative = format!(
            ".codex/plugins/cache/local-harness-plugins/harness-ultragoal/{}",
            registry.plugin_version()
        );
        let session_id = sha256(
            format!(
                "personal-marketplace-live-observation-v1\0{}\0{}",
                self.record.plan_sha256,
                registry.sha256()
            )
            .as_bytes(),
        );
        let source_tree =
            PersonalMarketplaceSourceObservation::capture(&self.home, &installed_root)
                .map_err(|_| "personal marketplace source observation failed")?;
        let installed =
            capture_installed_source_authority(&installed_root, registry.sha256(), &session_id)
                .map_err(|_| "personal marketplace installed source observation failed")?;
        let cache = capture_installed_source_authority(&cache_root, registry.sha256(), &session_id)
            .map_err(|_| "personal marketplace cache observation failed")?;
        let cache_tree = ScopedTree::new(self.home_root.clone(), &cache_relative)
            .and_then(|tree| tree.inspect(4096, 65 * 1024 * 1024))
            .map_err(|_| "personal marketplace cache tree observation failed")?
            .ok_or("personal marketplace cache tree is unavailable")?;
        let cache_tree_sha256 =
            tree_sha256(&cache_tree).map_err(|_| "personal marketplace cache tree is invalid")?;
        installed
            .revalidate()
            .map_err(|_| "personal marketplace installed source changed")?;
        cache
            .revalidate()
            .map_err(|_| "personal marketplace cache changed")?;
        source_tree
            .revalidate()
            .map_err(|_| "personal marketplace source tree changed")?;

        let tree = classify_digest(
            source_tree.tree_sha256(),
            &self.record.before.marketplace_source_tree_sha256,
            &self.record.target.source_tree_sha256,
        );
        let installed_match = classify_installed(
            installed.plugin_version(),
            installed.catalog_sha256(),
            &self.record.before.plugin_version,
            &self.record.before.marketplace_source_catalog_sha256,
            &self.record.target.version,
            tree,
        );
        let cache_match = classify_cache(
            cache.plugin_version(),
            cache.catalog_sha256(),
            &cache_tree_sha256,
            &self.record.before.plugin_version,
            &self.record.before.cache_catalog_sha256,
            &self.record.target.version,
            &self.record.target.source_tree_sha256,
        );
        let runtime = if installed.runtime_sha256() == self.record.before.marketplace_runtime_sha256
            && cache.runtime_sha256() == self.record.before.cache_runtime_sha256
        {
            PersonalMarketplaceAuthorityMatch::Prior
        } else if installed_match == PersonalMarketplaceAuthorityMatch::Target
            && cache_match == PersonalMarketplaceAuthorityMatch::Target
            && installed.runtime_sha256() == cache.runtime_sha256()
        {
            PersonalMarketplaceAuthorityMatch::Target
        } else {
            PersonalMarketplaceAuthorityMatch::Other
        };
        let registry_match = if registry.plugin_version() == self.record.before.plugin_version {
            PersonalMarketplaceAuthorityMatch::Prior
        } else if registry.plugin_version() == self.record.target.version {
            PersonalMarketplaceAuthorityMatch::Target
        } else {
            PersonalMarketplaceAuthorityMatch::Other
        };
        let marketplace_root_bound = registry.marketplace_root() == self.home
            && registry.installed_root() == installed_root
            && registry.selected_codex_identity_sha256()
                == self.record.before.selected_codex_identity_sha256;
        self.require_current()?;
        Ok(PersonalMarketplaceUpdateObservation {
            marketplace_root_bound,
            source_tree: tree,
            installed: installed_match,
            cache: cache_match,
            runtime,
            registry: registry_match,
        })
    }
}

impl PersonalMarketplaceUpdateEffects for LivePersonalMarketplaceUpdateEffects<'_> {
    fn cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    fn observe(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<PersonalMarketplaceUpdateObservation, &'static str> {
        self.require_authority(authority)?;
        self.capture_observation()
    }

    fn materialize_target(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str> {
        self.require_authority(authority)?;
        if self.materialization.is_some() {
            return Err("personal marketplace materialization replayed in one transaction");
        }
        let prior_tree = self
            .source_tree
            .inspect(4096, 65 * 1024 * 1024)
            .map_err(|_| "exact prior marketplace source snapshot failed")?
            .ok_or("exact prior marketplace source is unavailable")?;
        if tree_sha256(&prior_tree).map_err(|_| "exact prior marketplace source is invalid")?
            != self.record.before.marketplace_source_tree_sha256
        {
            return Err("exact prior marketplace source changed before materialization");
        }
        self.prior_tree = Some(prior_tree);
        let (_, transaction) = self
            .artifact
            .begin_marketplace_source_transition(
                self.source_context,
                &self.catalog,
                &mut self.source_tree,
                Some(&self.record.before.marketplace_source_tree_sha256),
            )
            .map_err(|_| "exact target marketplace source materialization failed")?;
        self.materialization = Some(transaction);
        self.require_current()
    }

    fn install_target(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str> {
        self.require_authority(authority)?;
        self.executable()?.install_personal_plugin(
            &self.package,
            super::MARKETPLACE,
            &self.home,
            &self.record.plan_sha256,
            &self.record.before.selected_codex_identity_sha256,
            &self.cancellation,
        )?;
        self.require_current()
    }

    fn restore_prior_tree(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str> {
        self.require_authority(authority)?;
        if let Some(transaction) = self.materialization.take() {
            self.artifact
                .rollback_marketplace_source_transition(
                    transaction,
                    &mut self.source_tree,
                    &self.record.before.marketplace_source_tree_sha256,
                )
                .map_err(|_| "exact prior marketplace source restoration failed")?;
        } else {
            let prior_tree = self
                .prior_tree
                .as_deref()
                .ok_or("exact prior marketplace source recovery snapshot is unavailable")?;
            restore_exact_prior_tree(
                &mut self.source_tree,
                &self.record.target.source_tree_sha256,
                prior_tree,
                &self.record.before.marketplace_source_tree_sha256,
            )?;
        }
        self.require_current()
    }

    fn reinstall_prior(
        &mut self,
        authority: &PersonalMarketplaceUpdateAuthority,
    ) -> Result<(), &'static str> {
        self.require_authority(authority)?;
        self.executable()?.reinstall_personal_plugin(
            &self.prior,
            super::MARKETPLACE,
            &self.home,
            &self.record.plan_sha256,
            &self.record.before.selected_codex_identity_sha256,
            &self.cancellation,
        )?;
        self.require_current()
    }
}

fn restore_exact_prior_tree(
    source_tree: &mut ScopedTree,
    expected_target_sha256: &str,
    prior_tree: &[TreeObject],
    expected_prior_sha256: &str,
) -> Result<(), &'static str> {
    if tree_sha256(prior_tree).map_err(|_| "exact prior recovery snapshot is invalid")?
        != expected_prior_sha256
    {
        return Err("exact prior recovery snapshot identity changed");
    }
    if !source_tree
        .compare_exchange_tree(Some(expected_target_sha256), Some(prior_tree))
        .map_err(|_| "exact prior marketplace source recovery failed")?
    {
        return Err("personal marketplace source changed before recovery");
    }
    let restored = source_tree
        .inspect(4096, 65 * 1024 * 1024)
        .map_err(|_| "restored personal marketplace source observation failed")?
        .ok_or("restored personal marketplace source is unavailable")?;
    if tree_sha256(&restored).map_err(|_| "restored personal marketplace source is invalid")?
        != expected_prior_sha256
    {
        return Err("restored personal marketplace source is not exact prior");
    }
    Ok(())
}

fn classify_installed(
    version: &str,
    catalog: &str,
    prior_version: &str,
    prior_catalog: &str,
    target_version: &str,
    tree: PersonalMarketplaceAuthorityMatch,
) -> PersonalMarketplaceAuthorityMatch {
    if version == prior_version && catalog == prior_catalog {
        PersonalMarketplaceAuthorityMatch::Prior
    } else if version == target_version && tree == PersonalMarketplaceAuthorityMatch::Target {
        PersonalMarketplaceAuthorityMatch::Target
    } else {
        PersonalMarketplaceAuthorityMatch::Other
    }
}

fn classify_digest(actual: &str, prior: &str, target: &str) -> PersonalMarketplaceAuthorityMatch {
    if actual == prior {
        PersonalMarketplaceAuthorityMatch::Prior
    } else if actual == target {
        PersonalMarketplaceAuthorityMatch::Target
    } else {
        PersonalMarketplaceAuthorityMatch::Other
    }
}

fn classify_cache(
    version: &str,
    catalog: &str,
    tree_sha256: &str,
    prior_version: &str,
    prior_catalog: &str,
    target_version: &str,
    target_tree_sha256: &str,
) -> PersonalMarketplaceAuthorityMatch {
    if version == prior_version && catalog == prior_catalog {
        PersonalMarketplaceAuthorityMatch::Prior
    } else if version == target_version && tree_sha256 == target_tree_sha256 {
        PersonalMarketplaceAuthorityMatch::Target
    } else {
        PersonalMarketplaceAuthorityMatch::Other
    }
}

fn directory_id(path: &Path) -> Result<String, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "personal home unavailable")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("personal home is not a directory");
    }
    Ok(sha256(
        format!("{}\0{}\0{}", path.display(), metadata.dev(), metadata.ino()).as_bytes(),
    ))
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

struct PersonalMarketplaceLease {
    path: PathBuf,
    file: File,
    device: u64,
    inode: u64,
}

impl PersonalMarketplaceLease {
    fn acquire(path: &Path) -> Result<Self, &'static str> {
        let named = fs::symlink_metadata(path).map_err(|_| "personal home unavailable")?;
        if named.file_type().is_symlink() || !named.is_dir() {
            return Err("personal home is not a directory");
        }
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
        let file = options
            .open(path)
            .map_err(|_| "personal marketplace lease descriptor unavailable")?;
        let opened = file
            .metadata()
            .map_err(|_| "personal marketplace lease descriptor unavailable")?;
        if opened.dev() != named.dev()
            || opened.ino() != named.ino()
            || unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        {
            return Err("personal marketplace update lease is busy or changed");
        }
        Ok(Self {
            path: path.to_path_buf(),
            file,
            device: opened.dev(),
            inode: opened.ino(),
        })
    }

    fn revalidate(&self) -> Result<(), &'static str> {
        let named = fs::symlink_metadata(&self.path)
            .map_err(|_| "personal marketplace lease root unavailable")?;
        let opened = self
            .file
            .metadata()
            .map_err(|_| "personal marketplace lease descriptor unavailable")?;
        if named.file_type().is_symlink()
            || !named.is_dir()
            || named.dev() != self.device
            || named.ino() != self.inode
            || opened.dev() != self.device
            || opened.ino() != self.inode
        {
            return Err("personal marketplace lease root changed");
        }
        Ok(())
    }
}

impl Drop for PersonalMarketplaceLease {
    fn drop(&mut self) {
        let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_product::lifecycle::Version;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_LEASE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn personal_marketplace_lease_is_exclusive_and_zero_persistence() {
        let root = std::env::temp_dir().join(format!(
            "hul-personal-marketplace-lease-{}-{}",
            std::process::id(),
            NEXT_LEASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let first = PersonalMarketplaceLease::acquire(&root).unwrap();
        assert!(PersonalMarketplaceLease::acquire(&root).is_err());
        first.revalidate().unwrap();
        drop(first);
        let second = PersonalMarketplaceLease::acquire(&root).unwrap();
        second.revalidate().unwrap();
        drop(second);
        assert!(fs::read_dir(&root).unwrap().next().is_none());
        fs::remove_dir(&root).unwrap();
    }

    #[test]
    fn authority_classification_rejects_mixed_values() {
        assert_eq!(
            classify_digest("sha256:other", "sha256:prior", "sha256:target"),
            PersonalMarketplaceAuthorityMatch::Other
        );
        assert_eq!(
            classify_installed(
                "0.0.42",
                "sha256:wrong",
                "0.0.41",
                "sha256:prior",
                "0.0.42",
                PersonalMarketplaceAuthorityMatch::Prior,
            ),
            PersonalMarketplaceAuthorityMatch::Other
        );
        assert_eq!(
            classify_cache(
                "0.0.42",
                "sha256:path-bound-cache-catalog",
                "sha256:exact-target-tree",
                "0.0.41",
                "sha256:path-bound-prior-catalog",
                "0.0.42",
                "sha256:exact-target-tree",
            ),
            PersonalMarketplaceAuthorityMatch::Target
        );
        assert_eq!(
            classify_cache(
                "0.0.42",
                "sha256:path-bound-cache-catalog",
                "sha256:other-tree",
                "0.0.41",
                "sha256:path-bound-prior-catalog",
                "0.0.42",
                "sha256:exact-target-tree",
            ),
            PersonalMarketplaceAuthorityMatch::Other
        );
    }

    #[test]
    fn interrupted_target_tree_restores_only_the_exact_prior_snapshot() {
        let requested = std::env::temp_dir().join(format!(
            "hul-distribution-personal-recovery-{}-{}",
            std::process::id(),
            NEXT_LEASE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&requested).unwrap();
        let requested = requested.canonicalize().unwrap();
        let root = ConfinedRoot::open(&requested).unwrap();
        let mut tree = ScopedTree::new(root.clone(), PERSONAL_MARKETPLACE_SOURCE_RELATIVE).unwrap();
        let prior = vec![TreeObject::regular(
            "prior.txt".to_owned(),
            0o644,
            b"exact prior".to_vec(),
        )];
        let target = vec![TreeObject::regular(
            "target.txt".to_owned(),
            0o644,
            b"exact target".to_vec(),
        )];
        let prior_sha256 = tree_sha256(&prior).unwrap();
        let target_sha256 = tree_sha256(&target).unwrap();
        assert!(tree.compare_exchange_tree(None, Some(&target)).unwrap());
        restore_exact_prior_tree(&mut tree, &target_sha256, &prior, &prior_sha256).unwrap();
        assert_eq!(tree.inspect(16, 1024).unwrap().unwrap(), prior);
        drop(tree);
        root.remove_owned().unwrap();
    }

    #[test]
    fn semver_target_is_distinct_from_prior() {
        assert_eq!(
            Version::parse("0.0.42")
                .unwrap()
                .precedence_cmp(&Version::parse("0.0.41").unwrap())
                .unwrap(),
            std::cmp::Ordering::Greater
        );
    }
}
