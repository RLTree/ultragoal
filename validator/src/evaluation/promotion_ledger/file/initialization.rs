impl FilePromotionReviewLedger {
    pub(crate) fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: PromotionLedgerBinding,
    ) -> Result<Self, PromotionLedgerError> {
        initialize_promotion_ledger(root.as_ref().to_path_buf(), key, binding)
    }

    pub(crate) fn open(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: PromotionLedgerBinding,
    ) -> Result<Self, PromotionLedgerError> {
        validate_binding(&binding)?;
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-unsafe"))?;
        let lock = open_review_lock(&root, false)?;
        let lock_identity = safe_file_identity(&lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-unsafe"))?;
        let anchor = open_review_anchor(&root, false)?;
        let anchor_authority = safe_file_identity(&anchor)
            .map_err(map_storage)?
            .authority();
        let key_id = sha256(&key);
        require_named_review_lock_identity(&root, lock_identity)?;
        require_named_review_anchor_authority(&root, anchor_authority)?;
        let (pending, current) = {
            let _guard = FileLock::exclusive(&lock)
                .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
            require_named_review_lock_identity(&root, lock_identity)?;
            require_named_review_anchor_authority(&root, anchor_authority)?;
            let pending = ensure_entries(&root_path, &root, false)?;
            let current = read_current(
                &root,
                &anchor,
                lock_identity,
                anchor_authority,
                &key,
                &key_id,
                &binding,
            )?;
            validate_pending_generation(pending.as_ref(), &current.snapshot)?;
            (pending, current)
        };
        let ledger = Self {
            root_path,
            root,
            root_identity,
            lock,
            lock_identity,
            anchor,
            anchor_authority,
            key,
            key_id,
            binding,
            expected_head: current.snapshot.head_sha256.clone(),
        };
        test_final_validation_pause(&ledger.root_path);
        let _guard = FileLock::exclusive(&ledger.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        ledger.require_unchanged_current(pending.as_ref(), &current)?;
        drop(_guard);
        Ok(ledger)
    }

    pub fn inspect(&self) -> Result<PromotionLedgerState, PromotionLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        let state = current.snapshot.payload.core.state.clone();
        test_final_validation_pause(&self.root_path);
        self.require_unchanged_current(pending.as_ref(), &current)?;
        Ok(state)
    }

    pub fn require_recovery(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), PromotionLedgerError> {
        let causal_code = causal_code.into();
        if !super::valid_identifier(&causal_code) {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-causal-code-invalid",
            ));
        }
        self.mutate(|state| match state {
            PromotionLedgerState::Ready | PromotionLedgerState::Issued { .. } => {
                Ok((PromotionLedgerState::RecoveryRequired { causal_code }, ()))
            }
            _ => Err(PromotionLedgerError::new(
                "promotion-ledger-recovery-transition-refused",
            )),
        })
    }
}
