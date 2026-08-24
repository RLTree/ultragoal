use super::super::{AnchoredDirectory, BOOTSTRAP_STAGE, STATE_COMPONENTS, validate_name};
use std::ffi::CString;
use std::os::fd::AsRawFd;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExclusivePublishSite {
    QuarantineSwappedLegacy,
    ExchangeLegacyAndFresh,
    BootstrapFresh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExclusivePublishFailure {
    BeforeRename,
    AfterRenameDurabilityUnknown,
}

impl ExclusivePublishSite {
    fn accepts(self, source: &str, destination: &str) -> bool {
        match self {
            Self::QuarantineSwappedLegacy => {
                source == BOOTSTRAP_STAGE && destination.starts_with("routine-public.quarantine-")
            }
            Self::ExchangeLegacyAndFresh => {
                source == STATE_COMPONENTS[3] && destination == BOOTSTRAP_STAGE
            }
            Self::BootstrapFresh => source == BOOTSTRAP_STAGE && destination == STATE_COMPONENTS[3],
        }
    }
}

#[cfg(test)]
thread_local! {
    static BEFORE_RENAME_FAILPOINT: std::cell::Cell<Option<ExclusivePublishSite>> = const { std::cell::Cell::new(None) };
    static AFTER_RENAME_FAILPOINT: std::cell::Cell<Option<ExclusivePublishSite>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn fail_before_next_rename(site: ExclusivePublishSite) {
    BEFORE_RENAME_FAILPOINT.with(|failpoint| failpoint.set(Some(site)));
}

#[cfg(test)]
pub(crate) fn fail_after_next_rename(site: ExclusivePublishSite) {
    AFTER_RENAME_FAILPOINT.with(|failpoint| failpoint.set(Some(site)));
}

#[cfg(test)]
fn take_after_rename_failpoint(site: ExclusivePublishSite) -> bool {
    AFTER_RENAME_FAILPOINT.with(|failpoint| {
        if failpoint.get() == Some(site) {
            failpoint.set(None);
            true
        } else {
            false
        }
    })
}

#[cfg(test)]
fn take_before_rename_failpoint(site: ExclusivePublishSite) -> bool {
    BEFORE_RENAME_FAILPOINT.with(|failpoint| {
        if failpoint.get() == Some(site) {
            failpoint.set(None);
            true
        } else {
            false
        }
    })
}

#[cfg(not(test))]
fn take_before_rename_failpoint(_site: ExclusivePublishSite) -> bool {
    false
}

#[cfg(not(test))]
fn take_after_rename_failpoint(_site: ExclusivePublishSite) -> bool {
    false
}

impl AnchoredDirectory {
    pub(crate) fn exchange_children(
        &self,
        left: &str,
        right: &str,
        site: ExclusivePublishSite,
    ) -> Result<(), ExclusivePublishFailure> {
        if site != ExclusivePublishSite::ExchangeLegacyAndFresh
            || !site.accepts(left, right)
            || validate_name(left).is_err()
            || validate_name(right).is_err()
        {
            return Err(ExclusivePublishFailure::BeforeRename);
        }
        let left_identity = self
            .stat(left)
            .map_err(|_| ExclusivePublishFailure::BeforeRename)?
            .ok_or(ExclusivePublishFailure::BeforeRename)?;
        let right_identity = self
            .stat(right)
            .map_err(|_| ExclusivePublishFailure::BeforeRename)?
            .ok_or(ExclusivePublishFailure::BeforeRename)?;
        if take_before_rename_failpoint(site) {
            return Err(ExclusivePublishFailure::BeforeRename);
        }
        let left_c = CString::new(left).map_err(|_| ExclusivePublishFailure::BeforeRename)?;
        let right_c = CString::new(right).map_err(|_| ExclusivePublishFailure::BeforeRename)?;
        const NOFOLLOW_AND_BENEATH: libc::c_uint = 0x10 | 0x20;
        // SAFETY: both names are validated, both descriptors are the same live
        // parent, and RENAME_SWAP changes both names in one namespace operation.
        let result = unsafe {
            libc::renameatx_np(
                self.file.as_raw_fd(),
                left_c.as_ptr(),
                self.file.as_raw_fd(),
                right_c.as_ptr(),
                libc::RENAME_SWAP | NOFOLLOW_AND_BENEATH,
            )
        };
        if result != 0 {
            return Err(ExclusivePublishFailure::BeforeRename);
        }
        if take_after_rename_failpoint(site)
            || self.file.sync_all().is_err()
            || self.stat(left).ok().flatten() != Some(right_identity)
            || self.stat(right).ok().flatten() != Some(left_identity)
        {
            return Err(ExclusivePublishFailure::AfterRenameDurabilityUnknown);
        }
        Ok(())
    }

    pub(crate) fn publish_child_exclusive(
        &self,
        source: &str,
        destination: &str,
        site: ExclusivePublishSite,
    ) -> Result<(), ExclusivePublishFailure> {
        if !site.accepts(source, destination)
            || validate_name(source).is_err()
            || validate_name(destination).is_err()
        {
            return Err(ExclusivePublishFailure::BeforeRename);
        }
        if take_before_rename_failpoint(site) {
            return Err(ExclusivePublishFailure::BeforeRename);
        }
        let source = CString::new(source).map_err(|_| ExclusivePublishFailure::BeforeRename)?;
        let destination =
            CString::new(destination).map_err(|_| ExclusivePublishFailure::BeforeRename)?;
        // SAFETY: both names are validated, both descriptors are the same live parent, and RENAME_EXCL prevents replacement.
        let result = unsafe {
            libc::renameatx_np(
                self.file.as_raw_fd(),
                source.as_ptr(),
                self.file.as_raw_fd(),
                destination.as_ptr(),
                libc::RENAME_EXCL,
            )
        };
        if result != 0 {
            return Err(ExclusivePublishFailure::BeforeRename);
        }
        if take_after_rename_failpoint(site) || self.file.sync_all().is_err() {
            return Err(ExclusivePublishFailure::AfterRenameDurabilityUnknown);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn parent() -> (std::path::PathBuf, AnchoredDirectory) {
        let root = std::env::temp_dir().join(format!(
            "hul-exclusive-publish-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let parent = AnchoredDirectory::open_absolute(
            &root,
            super::super::super::DirectorySecurity::PrivateAuthority,
        )
        .unwrap();
        (root, parent)
    }

    fn post_rename_failure_preserves_the_namespace_effect(
        source: &str,
        destination: &str,
        site: ExclusivePublishSite,
    ) {
        let (root, parent) = parent();
        let source_path = root.join(source);
        std::fs::create_dir(&source_path).unwrap();
        std::fs::set_permissions(&source_path, std::fs::Permissions::from_mode(0o700)).unwrap();
        fail_after_next_rename(site);

        assert_eq!(
            parent.publish_child_exclusive(source, destination, site),
            Err(ExclusivePublishFailure::AfterRenameDurabilityUnknown)
        );
        assert!(!source_path.exists());
        assert!(root.join(destination).is_dir());

        drop(parent);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn quarantine_publish_reports_post_rename_durability_ambiguity() {
        post_rename_failure_preserves_the_namespace_effect(
            BOOTSTRAP_STAGE,
            &format!("routine-public.quarantine-{}", "a".repeat(64)),
            ExclusivePublishSite::QuarantineSwappedLegacy,
        );
    }

    #[test]
    fn exchange_reports_post_rename_durability_ambiguity_without_a_missing_owner() {
        let (root, parent) = parent();
        for name in [STATE_COMPONENTS[3], BOOTSTRAP_STAGE] {
            let path = root.join(name);
            std::fs::create_dir(&path).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let legacy = parent.stat(STATE_COMPONENTS[3]).unwrap().unwrap();
        let fresh = parent.stat(BOOTSTRAP_STAGE).unwrap().unwrap();
        fail_after_next_rename(ExclusivePublishSite::ExchangeLegacyAndFresh);

        assert_eq!(
            parent.exchange_children(
                STATE_COMPONENTS[3],
                BOOTSTRAP_STAGE,
                ExclusivePublishSite::ExchangeLegacyAndFresh,
            ),
            Err(ExclusivePublishFailure::AfterRenameDurabilityUnknown)
        );
        assert_eq!(parent.stat(STATE_COMPONENTS[3]).unwrap(), Some(fresh));
        assert_eq!(parent.stat(BOOTSTRAP_STAGE).unwrap(), Some(legacy));

        drop(parent);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bootstrap_publish_reports_post_rename_durability_ambiguity() {
        post_rename_failure_preserves_the_namespace_effect(
            BOOTSTRAP_STAGE,
            STATE_COMPONENTS[3],
            ExclusivePublishSite::BootstrapFresh,
        );
    }
}
