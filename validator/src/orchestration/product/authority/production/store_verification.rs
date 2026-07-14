impl Store {
    pub(super) fn verify(&self) -> Result<(), ProductError> {
        verify_directory_path(&self.root, &self.directory)?;
        let metadata = self
            .directory
            .metadata()
            .map_err(|_| ProductError::AuthorityStoreInvalid)?;
        if metadata.dev() != self.root_device || metadata.ino() != self.root_inode {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        let mut names = fs::read_dir(&self.root)
            .map_err(|_| ProductError::AuthorityStoreInvalid)?
            .map(|entry| {
                entry
                    .map_err(|_| ProductError::AuthorityStoreInvalid)?
                    .file_name()
                    .into_string()
                    .map_err(|_| ProductError::AuthorityStoreInvalid)
            })
            .collect::<Result<Vec<_>, _>>()?;
        verify_directory_path(&self.root, &self.directory)?;
        names.sort();
        let mut expected = vec![ACTOR_NAME, KEY_NAME, LEDGER_NAME, LOCK_NAME];
        expected.sort();
        if names != expected {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        if read_bounded_at(&self.directory, KEY_NAME, 32)? != self.key
            || read_bounded_at(&self.directory, ACTOR_NAME, 256)? != self.actor_record
        {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        let lock = open_file_at(&self.directory, LOCK_NAME, libc::O_RDONLY, 256)?;
        self.verify_lock(&lock)?;
        let ledger = open_file_at(
            &self.directory,
            LEDGER_NAME,
            libc::O_RDONLY,
            MAX_LEDGER_BYTES,
        )?;
        self.verify_ledger(&ledger)
    }

    fn verify_lock(&self, lock: &File) -> Result<(), ProductError> {
        if file_identity(lock)? != (self.lock_device, self.lock_inode) {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        Ok(())
    }

    fn verify_ledger(&self, ledger: &File) -> Result<(), ProductError> {
        if file_identity(ledger)? != (self.ledger_device, self.ledger_inode) {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        Ok(())
    }
}
