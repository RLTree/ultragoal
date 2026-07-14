impl FilePromotionReviewLedger {
    fn require_unchanged_current(
        &self,
        expected_pending: Option<&PendingReviewPublication>,
        expected: &CurrentReviewSnapshot,
    ) -> Result<(), PromotionLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.as_ref() != expected_pending
            || current.snapshot.head_sha256 != expected.snapshot.head_sha256
            || current.observed_state_head_sha256 != expected.observed_state_head_sha256
            || current.observed_anchor != expected.observed_anchor
            || current.partial_tail_from != expected.partial_tail_from
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn require_published_current(
        &self,
        expected: &AuthenticatedReviewSnapshot,
    ) -> Result<(), PromotionLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.is_some()
            || current.partial_tail_from.is_some()
            || current.snapshot.head_sha256 != expected.head_sha256
            || current.observed_state_head_sha256 != expected.head_sha256
            || current.observed_anchor != expected.payload.anchor_observation
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }

    fn validate_descriptors(&self) -> Result<(), PromotionLedgerError> {
        let root = safe_file_identity(&self.root)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-stat-failed"))?;
        let lock = safe_file_identity(&self.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-stat-failed"))?;
        let anchor = safe_file_identity(&self.anchor).map_err(map_storage)?;
        let named_root = open_safe_directory(&self.root_path)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-descriptor-substituted"))?
            .1;
        if root.device != self.root_identity.device
            || root.inode != self.root_identity.inode
            || root.mode != self.root_identity.mode
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-root-descriptor-substituted",
            ));
        }
        if lock != self.lock_identity {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-lock-descriptor-substituted",
            ));
        }
        if anchor.authority() != self.anchor_authority {
            return Err(PromotionLedgerError::new(
                "promotion-anchor-descriptor-substituted",
            ));
        }
        require_named_review_lock_identity(&self.root, self.lock_identity)?;
        require_named_review_anchor_authority(&self.root, self.anchor_authority)?;
        if named_root.device != root.device
            || named_root.inode != root.inode
            || named_root.mode != root.mode
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-descriptor-substituted",
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set_test_publication_pause(root: PathBuf, milliseconds: u64) {
        publication_hooks()
            .lock()
            .expect("promotion publication hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_publication_is_paused(root: &Path) -> bool {
        publication_hooks()
            .lock()
            .expect("promotion publication hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_publication(root: &Path) {
        if let Some(hook) = publication_hooks()
            .lock()
            .expect("promotion publication hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_final_validation_pause(root: PathBuf, milliseconds: u64) {
        final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_final_validation_is_paused(root: &Path) -> bool {
        final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_final_validation(root: &Path) {
        if let Some(hook) = final_validation_hooks()
            .lock()
            .expect("promotion final validation hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_directory_scan_pause(root: PathBuf, milliseconds: u64) {
        directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock")
            .insert(root, TestPause::new(milliseconds));
    }

    #[cfg(test)]
    pub(crate) fn test_directory_scan_is_paused(root: &Path) -> bool {
        directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock")
            .get(root)
            .is_some_and(|hook| hook.paused)
    }

    #[cfg(test)]
    pub(crate) fn release_test_directory_scan(root: &Path) {
        if let Some(hook) = directory_scan_hooks()
            .lock()
            .expect("promotion directory scan hook lock")
            .get_mut(root)
        {
            hook.released = true;
        }
    }
}

#[cfg(test)]
struct TestPause {
    milliseconds: u64,
    paused: bool,
    released: bool,
}

#[cfg(test)]
impl TestPause {
    fn new(milliseconds: u64) -> Self {
        Self {
            milliseconds,
            paused: false,
            released: false,
        }
    }
}
