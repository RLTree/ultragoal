use super::*;

impl FitEffects for LocalEffects {
    fn compare_exchange(
        &mut self,
        path: &CanonicalPath,
        expected: &ExpectedContent,
        replacement: Option<&[u8]>,
    ) -> Result<bool, FitError> {
        if !self.mutation_lease {
            return Err(error(FitErrorId::Unauthorized));
        }
        self.injected_failure()?;
        if replacement.is_some_and(|bytes| {
            bytes.len() > crate::repository_fit::repository_contract::MAX_FILE_BYTES
        }) || matches!((expected, replacement), (ExpectedContent::Absent, None))
        {
            return Err(error(FitErrorId::InvalidSpec));
        }
        let create_parents = matches!(expected, ExpectedContent::Absent) && replacement.is_some();
        let (parent, created) = self.parent_anchor(path, create_parents)?;
        let name = path
            .components()
            .last()
            .expect("canonical path is nonempty");
        let observed = self.observe_leaf(&parent, name)?;
        let matches = match (expected, observed.as_ref()) {
            (ExpectedContent::Absent, None) => true,
            (ExpectedContent::ExactDigest(expected), Some(observed)) => {
                digest(&observed.bytes) == *expected
            }
            _ => false,
        };
        if !matches {
            self.cleanup_paths(&created)?;
            return Ok(false);
        }
        match (observed, replacement) {
            (None, Some(bytes)) => {
                self.insert_absent(&parent, name, bytes, created, self.desired_mode(path)?)
            }
            (Some(observed), Some(bytes)) => {
                debug_assert!(created.is_empty());
                let prior_mode = observed.mode;
                let rollback = match expected {
                    ExpectedContent::ExactDigest(expected) => self
                        .rollback_modes
                        .get(path.as_str())
                        .filter(|(current, _)| current == expected)
                        .cloned(),
                    ExpectedContent::Absent => None,
                };
                let replacement_mode = rollback
                    .as_ref()
                    .map(|(_, mode)| *mode)
                    .or_else(|| self.unix_modes.get(path.as_str()).copied())
                    .ok_or_else(|| error(FitErrorId::InvalidSpec))?;
                let changed =
                    self.replace_existing(&parent, name, observed, bytes, replacement_mode)?;
                if changed {
                    if rollback.is_some() {
                        self.rollback_modes.remove(path.as_str());
                    } else {
                        self.rollback_modes
                            .insert(path.as_str().to_owned(), (digest(bytes), prior_mode));
                    }
                }
                Ok(changed)
            }
            (Some(observed), None) => {
                debug_assert!(created.is_empty());
                self.remove_existing(&parent, name, observed)
            }
            (None, None) => Err(error(FitErrorId::InvalidSpec)),
        }
    }
}

pub(crate) fn bounded_read(file: &mut File) -> Result<Vec<u8>, FitError> {
    let maximum = crate::repository_fit::repository_contract::MAX_FILE_BYTES;
    let mut bytes = Vec::new();
    Read::by_ref(file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(FitErrorId::ReadFailed))?;
    if bytes.len() > maximum {
        return Err(error(FitErrorId::ResourceLimit));
    }
    Ok(bytes)
}

pub(crate) fn object_identity(metadata: &fs::Metadata) -> ObjectIdentity {
    ObjectIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

pub(crate) fn require_same_device(root_device: u64, object_device: u64) -> Result<(), FitError> {
    if root_device == object_device {
        Ok(())
    } else {
        Err(error(FitErrorId::UnsafeObject))
    }
}

pub(crate) const fn stat_identity(stat: PathStat) -> ObjectIdentity {
    ObjectIdentity {
        device: stat.device,
        inode: stat.inode,
    }
}

pub(crate) fn leaf_stat(metadata: &fs::Metadata) -> PathStat {
    PathStat {
        device: metadata.dev(),
        inode: metadata.ino(),
        links: metadata.nlink(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
        regular: metadata.is_file(),
    }
}

pub(crate) fn temp_name() -> String {
    format!(
        ".hul-fit-{:x}-{:x}",
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    )
}

pub(crate) fn transaction_name() -> Result<String, FitError> {
    random_name(".hul-fit-transaction-")
}

pub(crate) fn directory_temp_name() -> Result<String, FitError> {
    random_name(".hul-fit-directory-")
}

pub(crate) fn random_name(prefix: &str) -> Result<String, FitError> {
    let mut random = [0u8; 16];
    // SAFETY: `random` is valid writable memory for exactly the requested byte count.
    if unsafe {
        libc::getentropy(
            random.as_mut_ptr().cast::<libc::c_void>(),
            random.len() as libc::size_t,
        )
    } != 0
    {
        return Err(error(FitErrorId::UnsupportedHost));
    }
    let mut encoded = String::with_capacity(32);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").map_err(|_| error(FitErrorId::EffectFailed))?;
    }
    Ok(format!("{prefix}{encoded}"))
}
