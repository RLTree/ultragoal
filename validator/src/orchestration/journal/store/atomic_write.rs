impl Store {
    pub(crate) fn write_atomic(&self, name: &str, bytes: &[u8]) -> Result<(), OrchestrationError> {
        self.verify_root()?;
        let target_before = self.exact_stat(name)?;
        if let Some(target) = target_before {
            if !target.regular || target.links != 1 || target.length > MAX_JOURNAL_BYTES {
                return Err(OrchestrationError::JournalCorrupt);
            }
        }
        let (temp_name, mut file) = self.create_temp(name)?;
        let result = (|| {
            file.write_all(bytes)
                .map_err(|_| OrchestrationError::JournalIo)?;
            file.sync_all().map_err(|_| OrchestrationError::JournalIo)?;
            self.verify_root()?;
            self.sync_root()?;
            self.verify_root()?;
            if self.exact_stat(name)? != target_before {
                return Err(OrchestrationError::JournalConflict);
            }
            #[cfg(test)]
            super::test_hook::run();
            super::sys::rename_relative(&self.directory, &temp_name, name)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = super::sys::unlink_relative(&self.directory, &temp_name);
        }
        result
    }

    pub(crate) fn sync_root(&self) -> Result<(), OrchestrationError> {
        self.directory
            .sync_all()
            .map_err(|_| OrchestrationError::JournalIo)
    }

    fn create_temp(&self, target: &str) -> Result<(String, File), OrchestrationError> {
        for attempt in 0..64_u8 {
            let name = format!(".{target}.{}.{}.tmp", std::process::id(), attempt);
            match super::sys::open_relative(
                &self.directory,
                &name,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            ) {
                Ok(file) if self.exact_entry(&name)? => return Ok((name, file)),
                Ok(_) => return Err(OrchestrationError::JournalCorrupt),
                Err(OrchestrationError::JournalConflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(OrchestrationError::JournalIo)
    }

    fn exact_entry(&self, name: &str) -> Result<bool, OrchestrationError> {
        self.verify_root()?;
        let exact = super::sys::exact_entry(&self.directory, name)?;
        self.verify_root()?;
        Ok(exact)
    }

    fn exact_stat(
        &self,
        name: &str,
    ) -> Result<Option<super::sys::RelativeStat>, OrchestrationError> {
        if !self.exact_entry(name)? {
            return Ok(None);
        }
        let stat = super::sys::relative_stat(&self.directory, name)?
            .ok_or(OrchestrationError::JournalCorrupt)?;
        if !self.exact_entry(name)? {
            return Err(OrchestrationError::JournalCorrupt);
        }
        Ok(Some(stat))
    }
}
