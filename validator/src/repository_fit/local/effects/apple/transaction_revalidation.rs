use super::*;

impl LocalEffects {
    pub(crate) fn private_transaction_is_current(
        &self,
        transaction: &PrivateTransaction,
    ) -> Result<(), FitError> {
        self.verify_root()?;
        let mut budget = EnumerationBudget::new();
        let stat = stat_at(&self.root, &transaction.name)?
            .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
        if exact_entry(&self.root, &transaction.name, &mut budget)? != EntryMatch::Exact
            || stat_identity(stat) != transaction.identity
            || descriptor_path(&transaction.directory).ok() != Some(transaction.canonical.clone())
        {
            return Err(error(FitErrorId::RollbackFailed));
        }
        Ok(())
    }
    pub(crate) fn close_private_transaction(
        &self,
        transaction: PrivateTransaction,
    ) -> Result<(), FitError> {
        self.private_transaction_is_current(&transaction)?;
        unlink_at(&self.root, &transaction.name, libc::AT_REMOVEDIR)
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        self.root
            .sync_all()
            .map_err(|_| error(FitErrorId::RollbackFailed))?;
        Ok(())
    }
    pub(crate) fn cleanup_created_directories(&mut self) -> Result<(), FitError> {
        let mut paths = self
            .created_directories
            .iter()
            .map(|(path, identity)| CreatedDirectory {
                path: path.clone(),
                identity: *identity,
            })
            .collect::<Vec<_>>();
        paths.sort_by_key(|row| std::cmp::Reverse(row.path.matches('/').count()));
        for row in paths {
            match self.remove_directory(&row) {
                Ok(true) => {
                    self.created_directories.remove(&row.path);
                }
                Ok(false) => {}
                Err(failure) => return Err(failure),
            }
        }
        Ok(())
    }
    pub(crate) fn cleanup_paths(&mut self, paths: &[CreatedDirectory]) -> Result<(), FitError> {
        for row in paths.iter().rev() {
            let _ = self.remove_directory(row)?;
        }
        Ok(())
    }
    pub(crate) fn remove_directory(
        &mut self,
        created: &CreatedDirectory,
    ) -> Result<bool, FitError> {
        let marker = match created.path.rsplit_once('/') {
            Some((parent, _)) => CanonicalPath::parse(format!("{parent}/.hul-cleanup"))?,
            None => CanonicalPath::parse(".hul-cleanup")?,
        };
        let (parent, _) = self.parent_anchor(&marker, false)?;
        let name = created
            .path
            .rsplit('/')
            .next()
            .ok_or_else(|| error(FitErrorId::InvalidPath))?;
        let mut budget = EnumerationBudget::new();
        if exact_entry(&parent.directory, name, &mut budget)? != EntryMatch::Exact {
            return Err(error(FitErrorId::StaleBinding));
        }
        let stat =
            stat_at(&parent.directory, name)?.ok_or_else(|| error(FitErrorId::RollbackFailed))?;
        if stat_identity(stat) != created.identity {
            return Err(error(FitErrorId::RollbackFailed));
        }
        self.quarantine_remove_directory(&parent, name, created.identity)
    }
    pub(crate) fn desired_mode(&self, path: &CanonicalPath) -> Result<u32, FitError> {
        self.unix_modes
            .get(path.as_str())
            .copied()
            .ok_or_else(|| error(FitErrorId::InvalidSpec))
    }
    #[cfg(test)]
    pub(crate) fn injected_failure(&mut self) -> Result<(), FitError> {
        self.compare_calls += 1;
        if self.fail_calls.contains(&self.compare_calls) {
            Err(error(FitErrorId::EffectFailed))
        } else {
            Ok(())
        }
    }
    #[cfg(not(test))]
    pub(crate) const fn injected_failure(&mut self) -> Result<(), FitError> {
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn test_before_linearize(&mut self) {
        if let Some((reached, resume)) = self.before_linearize.take() {
            reached.wait();
            resume.wait();
        }
    }
    #[cfg(not(test))]
    pub(crate) const fn test_before_linearize(&mut self) {}
    #[cfg(test)]
    pub(crate) fn test_after_replace_swap(&mut self) {
        if let Some((reached, resume)) = self.after_replace_swap.take() {
            reached.wait();
            resume.wait();
        }
    }
    #[cfg(not(test))]
    pub(crate) const fn test_after_replace_swap(&mut self) {}
    #[cfg(test)]
    pub(crate) fn test_before_quarantine_move(&mut self) {
        if let Some((reached, resume)) = self.before_quarantine_move.take() {
            reached.wait();
            resume.wait();
        }
    }
    #[cfg(not(test))]
    pub(crate) const fn test_before_quarantine_move(&mut self) {}
    #[cfg(test)]
    pub(crate) fn test_after_directory_publish(&mut self) {
        if let Some((reached, resume)) = self.after_directory_publish.take() {
            reached.wait();
            resume.wait();
        }
    }
    #[cfg(not(test))]
    pub(crate) const fn test_after_directory_publish(&mut self) {}
    #[cfg(test)]
    pub(crate) fn test_before_directory_quarantine_move(&mut self) {
        if let Some((reached, resume)) = self.before_directory_quarantine_move.take() {
            reached.wait();
            resume.wait();
        }
    }
    #[cfg(not(test))]
    pub(crate) const fn test_before_directory_quarantine_move(&mut self) {}
}

impl FitReader for LocalEffects {
    fn root_binding(&mut self) -> Result<String, FitError> {
        self.verify_root()?;
        if self.reader.root_binding()? != self.binding {
            return Err(error(FitErrorId::StaleBinding));
        }
        Ok(self.binding.clone())
    }

    fn read_file(
        &mut self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.verify_root()?;
        self.reader.read_file(path, maximum_bytes)
    }
}
