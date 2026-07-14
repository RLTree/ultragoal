use super::*;

impl LocalEffects {
    pub(crate) fn remove_existing(
        &mut self,
        parent: &ParentAnchor,
        name: &str,
        observed: ObservedLeaf,
    ) -> Result<bool, FitError> {
        let temp = temp_name();
        self.test_before_linearize();
        rename_exclusive(&parent.directory, name, &temp)?;
        let moved = stat_at(&parent.directory, &temp)?;
        let valid = moved.is_some_and(|stat| stat_identity(stat) == stat_identity(observed.stat))
            && stat_at(&parent.directory, name)?.is_none()
            && descriptor_path(&observed.file).ok() == Some(parent.canonical.join(&temp))
            && self.verify_parent(parent).is_ok();
        if !valid {
            return Err(error(FitErrorId::RollbackFailed));
        }
        self.quarantine_unlink(parent, &temp, stat_identity(observed.stat), &observed.file)?;
        self.cleanup_created_directories()?;
        self.verify_root()?;
        Ok(true)
    }
    pub(crate) fn quarantine_unlink(
        &mut self,
        source_parent: &ParentAnchor,
        source_name: &str,
        expected: ObjectIdentity,
        held: &File,
    ) -> Result<(), FitError> {
        let transaction = self.open_private_transaction()?;
        self.test_before_quarantine_move();
        if rename_between(
            &source_parent.directory,
            source_name,
            &transaction.directory,
            "object",
            libc::RENAME_EXCL,
        )
        .is_err()
        {
            let _ = self.close_private_transaction(transaction);
            return Err(error(FitErrorId::RollbackFailed));
        }
        let mut budget = EnumerationBudget::new();
        let moved = stat_at(&transaction.directory, "object")?;
        let valid = moved.is_some_and(|stat| stat_identity(stat) == expected)
            && exact_entry(&transaction.directory, "object", &mut budget)? == EntryMatch::Exact
            && descriptor_path(held).ok() == Some(transaction.canonical.join("object"))
            && self.private_transaction_is_current(&transaction).is_ok();
        if !valid {
            return Err(error(FitErrorId::RollbackFailed));
        }
        unlink_at(&transaction.directory, "object", 0)
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        transaction
            .directory
            .sync_all()
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        self.close_private_transaction(transaction)
    }
    pub(crate) fn quarantine_remove_directory(
        &mut self,
        source_parent: &ParentAnchor,
        source_name: &str,
        expected: ObjectIdentity,
    ) -> Result<bool, FitError> {
        let held = open_at(
            &source_parent.directory,
            source_name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )?
        .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
        self.quarantine_remove_directory_from(
            &source_parent.directory,
            &source_parent.canonical,
            source_name,
            expected,
            &held,
        )
    }
    pub(crate) fn quarantine_remove_directory_from(
        &mut self,
        source_parent: &File,
        source_parent_canonical: &Path,
        source_name: &str,
        expected: ObjectIdentity,
        held: &File,
    ) -> Result<bool, FitError> {
        let metadata = held
            .metadata()
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        let mut budget = EnumerationBudget::new();
        if !metadata.is_dir()
            || object_identity(&metadata) != expected
            || descriptor_path(held).ok() != Some(source_parent_canonical.join(source_name))
            || exact_entry(source_parent, source_name, &mut budget)? != EntryMatch::Exact
            || stat_at(source_parent, source_name)?
                .is_none_or(|stat| stat_identity(stat) != expected)
        {
            return Err(error(FitErrorId::RollbackFailed));
        }
        if !directory_is_empty(held)? {
            return Ok(false);
        }
        let transaction = self.open_private_transaction()?;
        self.test_before_directory_quarantine_move();
        if rename_between(
            source_parent,
            source_name,
            &transaction.directory,
            "object",
            libc::RENAME_EXCL,
        )
        .is_err()
        {
            let _ = self.close_private_transaction(transaction);
            return Err(error(FitErrorId::RollbackFailed));
        }
        let moved = stat_at(&transaction.directory, "object")?;
        let mut budget = EnumerationBudget::new();
        let valid = moved.is_some_and(|stat| stat_identity(stat) == expected)
            && exact_entry(&transaction.directory, "object", &mut budget)? == EntryMatch::Exact
            && descriptor_path(held).ok() == Some(transaction.canonical.join("object"))
            && self.private_transaction_is_current(&transaction).is_ok();
        if !valid {
            return Err(error(FitErrorId::RollbackFailed));
        }
        unlink_at(&transaction.directory, "object", libc::AT_REMOVEDIR)
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        transaction
            .directory
            .sync_all()
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        self.close_private_transaction(transaction)?;
        Ok(true)
    }
    pub(crate) fn open_private_transaction(&self) -> Result<PrivateTransaction, FitError> {
        self.verify_root()?;
        for _ in 0..8 {
            let name = transaction_name()?;
            let mut budget = EnumerationBudget::new();
            match exact_entry(&self.root, &name, &mut budget)? {
                EntryMatch::Absent => {}
                EntryMatch::Exact | EntryMatch::Alias => continue,
            }
            match mkdir_at(&self.root, &name, 0o700) {
                Ok(()) => {}
                Err(failure) if failure.id() == FitErrorId::Conflict => continue,
                Err(failure) => return Err(failure),
            }
            let directory = open_at(
                &self.root,
                &name,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?
            .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
            let metadata = directory
                .metadata()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            let identity = object_identity(&metadata);
            let canonical = self.canonical.join(&name);
            let stat = stat_at(&self.root, &name)?;
            if !metadata.is_dir()
                || require_same_device(self.root_identity.device, metadata.dev()).is_err()
                || stat.is_none_or(|stat| stat_identity(stat) != identity)
                || descriptor_path(&directory).ok() != Some(canonical.clone())
                || unsafe { libc::fchmod(directory.as_raw_fd(), 0o700 as libc::mode_t) } != 0
                || directory.sync_all().is_err()
            {
                return Err(error(FitErrorId::RollbackFailed));
            }
            let transaction = PrivateTransaction {
                name,
                directory,
                canonical,
                identity,
            };
            self.private_transaction_is_current(&transaction)?;
            return Ok(transaction);
        }
        Err(error(FitErrorId::ResourceLimit))
    }
}
