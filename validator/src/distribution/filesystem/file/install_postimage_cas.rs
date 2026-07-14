#[cfg(unix)]
impl ScopedFile {
    pub(crate) fn apply_with_postimage(
        &self,
        expected_sha256: Option<&str>,
        replacement: Option<&[u8]>,
    ) -> Result<(bool, Option<InstalledPostimage>), DistributionError> {
        let Some(replacement_bytes) = replacement else {
            return self.apply(expected_sha256, None).map(|changed| (changed, None));
        };
        self.apply_replacement(expected_sha256, replacement_bytes)
    }

    pub(crate) fn apply_postimage(
        &self,
        expected: &InstalledPostimage,
        replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        if replacement.is_some_and(|row| row.len() > FILE_LIMIT) {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        if !expected.matches_target(self.root_id(), self.relative_path()) {
            return Err(error(DistributionErrorId::ProvenanceMismatch));
        }
        let root = self.root.root_directory()?;
        let lock_name = format!(".hul-lock-{}", &sha256(self.relative.as_bytes())[7..]);
        let _lock = OwnedFile::create(root.duplicate()?, lock_name, &[], 0o600, true)?;
        let (parent, name) = self.root.parent(&self.relative, true)?;
        let before = read_file(&parent, &name, FILE_LIMIT)?;
        let Some(before) = before else {
            return Ok(false);
        };
        let before_postimage = postimage_from_parts(
            self.root_id(),
            self.relative_path(),
            &before.bytes,
            before.identity,
            before.mode,
        );
        if &before_postimage != expected {
            return Ok(false);
        }
        let stage_name = format!(
            ".hul-stage-{}-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed),
            &sha256(self.relative.as_bytes())[7..],
        );
        let mut stage = replacement
            .map(|bytes| {
                OwnedFile::create(root.duplicate()?, stage_name.clone(), bytes, 0o600, false)
            })
            .transpose()?;
        let current = read_file(&parent, &name, FILE_LIMIT)?;
        if !same_snapshot(current.as_ref(), Some(&before)) {
            return Ok(false);
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let transitioned = transition(
            &root,
            &parent,
            &name,
            &stage_name,
            Some(&before),
            replacement,
            stage.as_mut(),
        )?;
        if !transitioned {
            return Ok(false);
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let after = read_file(&parent, &name, FILE_LIMIT)?;
        if after.as_ref().map(|row| row.bytes.as_slice()) != replacement {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(true)
    }

    fn apply_replacement(
        &self,
        expected_sha256: Option<&str>,
        replacement: &[u8],
    ) -> Result<(bool, Option<InstalledPostimage>), DistributionError> {
        if replacement.len() > FILE_LIMIT {
            return Err(error(DistributionErrorId::ObjectTooLarge));
        }
        let root = self.root.root_directory()?;
        let lock_name = format!(".hul-lock-{}", &sha256(self.relative.as_bytes())[7..]);
        let _lock = OwnedFile::create(root.duplicate()?, lock_name, &[], 0o600, true)?;
        let (parent, name) = self.root.parent(&self.relative, true)?;
        let before = read_file(&parent, &name, FILE_LIMIT)?;
        if before.as_ref().map(|row| sha256(&row.bytes)).as_deref() != expected_sha256 {
            return Ok((false, None));
        }
        let stage_name = format!(
            ".hul-stage-{}-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed),
            &sha256(self.relative.as_bytes())[7..],
        );
        let mut stage = OwnedFile::create(
            root.duplicate()?,
            stage_name.clone(),
            replacement,
            0o600,
            false,
        )?;
        let current = read_file(&parent, &name, FILE_LIMIT)?;
        if !same_snapshot(current.as_ref(), before.as_ref()) {
            return Ok((false, None));
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let transitioned = transition(
            &root,
            &parent,
            &name,
            &stage_name,
            before.as_ref(),
            Some(replacement),
            Some(&mut stage),
        )?;
        if !transitioned {
            return Ok((false, None));
        }
        self.root.revalidate_parent(&self.relative, &parent)?;
        let after = read_file(&parent, &name, FILE_LIMIT)?
            .ok_or_else(|| error(DistributionErrorId::ObjectChanged))?;
        if !after.identity.same_after_rename(stage.identity())
            || after.mode != stage.mode()
            || after.bytes != replacement
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok((
            true,
            Some(postimage_from_parts(
                self.root_id(),
                self.relative_path(),
                replacement,
                after.identity,
                after.mode,
            )),
        ))
    }
}

#[cfg(not(unix))]
impl ScopedFile {
    pub(crate) fn apply_postimage(
        &self,
        _expected: &InstalledPostimage,
        _replacement: Option<&[u8]>,
    ) -> Result<bool, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    pub(crate) fn apply_with_postimage(
        &self,
        _expected_sha256: Option<&str>,
        _replacement: Option<&[u8]>,
    ) -> Result<(bool, Option<InstalledPostimage>), DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }
}

#[cfg(unix)]
fn postimage_from_parts(
    root_id: &str,
    relative: &str,
    bytes: &[u8],
    identity: super::descriptor::FileIdentity,
    mode: u32,
) -> InstalledPostimage {
    InstalledPostimage::new(
        root_id.into(),
        relative,
        sha256(bytes),
        sha256(
            format!(
                "{}\0{}\0{}\0{}\0{}\0{}\0{}\0{}",
                identity.device,
                identity.inode,
                identity.length,
                identity.modified_seconds,
                identity.modified_nanos,
                identity.changed_seconds,
                identity.changed_nanos,
                mode
            )
            .as_bytes(),
        ),
        identity.length,
        mode,
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::distribution::filesystem::ConfinedRoot;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn exact_postimage_cas_rejects_same_bytes_different_object() {
        let root = test_root("postimage-cas");
        std::fs::create_dir_all(root.join("plugins")).unwrap();
        let confined = ConfinedRoot::open(&root).unwrap();
        let file = ScopedFile::new(confined, "plugins/harness-ultragoal.hugpkg").unwrap();
        let bytes = b"package bytes";
        assert!(file.apply(None, Some(bytes)).unwrap());
        let expected = file.installed_postimage(FILE_LIMIT).unwrap().unwrap();

        std::fs::remove_file(root.join("plugins/harness-ultragoal.hugpkg")).unwrap();
        std::fs::write(root.join("plugins/harness-ultragoal.hugpkg"), bytes).unwrap();

        assert!(!file.apply_postimage(&expected, None).unwrap());
        assert_eq!(
            std::fs::read(root.join("plugins/harness-ultragoal.hugpkg")).unwrap(),
            bytes
        );
        let _ = std::fs::remove_dir_all(root);
    }

    fn test_root(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "hul-distribution-{label}-{}-{}",
            std::process::id(),
            NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
