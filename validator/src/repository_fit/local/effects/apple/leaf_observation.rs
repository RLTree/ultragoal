use super::*;

impl LocalEffects {
    pub(crate) fn observe_leaf(
        &self,
        parent: &ParentAnchor,
        name: &str,
    ) -> Result<Option<ObservedLeaf>, FitError> {
        self.verify_parent(parent)?;
        let mut budget = EnumerationBudget::new();
        match exact_entry(&parent.directory, name, &mut budget)? {
            EntryMatch::Absent => {
                if stat_at(&parent.directory, name)?.is_some() {
                    return Err(error(FitErrorId::StaleBinding));
                }
                return Ok(None);
            }
            EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
            EntryMatch::Exact => {}
        }
        let before =
            stat_at(&parent.directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
        if !before.regular
            || before.links != 1
            || require_same_device(self.root_identity.device, before.device).is_err()
            || before.length > crate::repository_fit::repository_contract::MAX_FILE_BYTES as u64
        {
            return Err(error(
                if before.length > crate::repository_fit::repository_contract::MAX_FILE_BYTES as u64
                {
                    FitErrorId::ResourceLimit
                } else {
                    FitErrorId::UnsafeObject
                },
            ));
        }
        let mut file = open_at(
            &parent.directory,
            name,
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )?
        .ok_or_else(|| error(FitErrorId::StaleBinding))?;
        let metadata = file.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
        if leaf_stat(&metadata) != before {
            return Err(error(FitErrorId::StaleBinding));
        }
        let first = bounded_read(&mut file)?;
        file.seek(SeekFrom::Start(0))
            .map_err(|_| error(FitErrorId::ReadFailed))?;
        let second = bounded_read(&mut file)?;
        let after =
            stat_at(&parent.directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
        let expected_path = parent.canonical.join(name);
        if first != second
            || first.len() as u64 != before.length
            || before != after
            || exact_entry(&parent.directory, name, &mut budget)? != EntryMatch::Exact
            || descriptor_path(&file)? != expected_path
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        self.verify_parent(parent)?;
        Ok(Some(ObservedLeaf {
            file,
            stat: before,
            bytes: first,
            mode: metadata.permissions().mode() & 0o7777,
        }))
    }
    pub(crate) fn create_replacement(
        &mut self,
        parent: &ParentAnchor,
        _name: &str,
        bytes: &[u8],
        mode: u32,
    ) -> Result<(String, File, ObjectIdentity), FitError> {
        let temp_name = temp_name();
        let mut budget = EnumerationBudget::new();
        if exact_entry(&parent.directory, &temp_name, &mut budget)? != EntryMatch::Absent {
            return Err(error(FitErrorId::UnsafeObject));
        }
        let mut file = create_file_at(&parent.directory, &temp_name, mode)?;
        // SAFETY: `file` owns the newly created descriptor and `mode` is the requested permission mask.
        let write_succeeded = unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) } == 0
            && file.write_all(bytes).and_then(|()| file.sync_all()).is_ok();
        let metadata = file
            .metadata()
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        let identity = object_identity(&metadata);
        if !write_succeeded
            || !metadata.is_file()
            || metadata.nlink() != 1
            || require_same_device(self.root_identity.device, metadata.dev()).is_err()
            || metadata.len() != bytes.len() as u64
            || metadata.permissions().mode() & 0o7777 != mode
            || descriptor_path(&file).ok() != Some(parent.canonical.join(&temp_name))
        {
            return Err(error(
                if self
                    .quarantine_unlink(parent, &temp_name, identity, &file)
                    .is_ok()
                {
                    FitErrorId::EffectFailed
                } else {
                    FitErrorId::RollbackFailed
                },
            ));
        }
        Ok((temp_name, file, identity))
    }
    pub(crate) fn insert_absent(
        &mut self,
        parent: &ParentAnchor,
        name: &str,
        replacement: &[u8],
        created: Vec<CreatedDirectory>,
        mode: u32,
    ) -> Result<bool, FitError> {
        let (temp, file, identity) = self.create_replacement(parent, name, replacement, mode)?;
        self.test_before_linearize();
        match rename_exclusive(&parent.directory, &temp, name) {
            Ok(()) => {}
            Err(failure) if failure.id() == FitErrorId::Conflict => {
                self.quarantine_unlink(parent, &temp, identity, &file)?;
                self.cleanup_paths(&created)?;
                return Ok(false);
            }
            Err(failure) => {
                self.quarantine_unlink(parent, &temp, identity, &file)?;
                self.cleanup_paths(&created)?;
                return Err(failure);
            }
        }
        let active = stat_at(&parent.directory, name)?;
        if active.is_none_or(|stat| stat_identity(stat) != identity)
            || descriptor_path(&file)? != parent.canonical.join(name)
            || self.verify_parent(parent).is_err()
        {
            return Err(error(FitErrorId::RollbackFailed));
        }
        self.created_directories
            .extend(created.into_iter().map(|row| (row.path, row.identity)));
        Ok(true)
    }
    pub(crate) fn replace_existing(
        &mut self,
        parent: &ParentAnchor,
        name: &str,
        observed: ObservedLeaf,
        replacement: &[u8],
        replacement_mode: u32,
    ) -> Result<bool, FitError> {
        let (temp, next, next_identity) =
            self.create_replacement(parent, name, replacement, replacement_mode)?;
        self.test_before_linearize();
        if let Err(failure) = rename_swap(&parent.directory, &temp, name) {
            self.quarantine_unlink(parent, &temp, next_identity, &next)?;
            return Err(failure);
        }
        self.test_after_replace_swap();
        let swapped_prior = stat_at(&parent.directory, &temp)?;
        let active = stat_at(&parent.directory, name)?;
        let valid = swapped_prior
            .is_some_and(|stat| stat_identity(stat) == stat_identity(observed.stat))
            && active.is_some_and(|stat| stat_identity(stat) == next_identity)
            && descriptor_path(&observed.file).ok() == Some(parent.canonical.join(&temp))
            && descriptor_path(&next).ok() == Some(parent.canonical.join(name))
            && self.verify_parent(parent).is_ok();
        if !valid {
            return Err(error(FitErrorId::RollbackFailed));
        }
        self.quarantine_unlink(parent, &temp, stat_identity(observed.stat), &observed.file)?;
        Ok(true)
    }
}
