impl FilePromotionReviewLedger {
    pub(crate) fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: PromotionLedgerBinding,
    ) -> Result<Self, PromotionLedgerError> {
        validate_binding(&binding)?;
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-root-unsafe"))?;
        ensure_entries(&root_path, &root, true)?;
        let lock = open_review_lock(&root, true)?;
        let lock_identity = safe_file_identity(&lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-unsafe"))?;
        let anchor = open_review_anchor(&root, true)?;
        let anchor_authority = safe_file_identity(&anchor)
            .map_err(map_storage)?
            .authority();
        let key_id = sha256(&key);
        let core = ReviewSnapshotCore {
            schema_version: "PromotionReviewLedger-v1".to_owned(),
            generation: 0,
            previous_head_sha256: sha256(b"promotion-review-genesis"),
            key_id: key_id.clone(),
            lock_identity,
            anchor_authority,
            binding: binding.clone(),
            state: PromotionLedgerState::Ready,
        };
        let record = authenticate_anchor_record(
            ReviewAnchorRecordPayload {
                schema_version: "PromotionReviewAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: sha256(ANCHOR_GENESIS),
                core: core.clone(),
            },
            &key,
        )?;
        require_named_review_lock_identity(&root, lock_identity)?;
        require_named_review_anchor_authority(&root, anchor_authority)?;
        let _guard = FileLock::exclusive(&lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        require_named_review_lock_identity(&root, lock_identity)?;
        require_named_review_anchor_authority(&root, anchor_authority)?;
        if entry_exists(&root, STATE_NAME).map_err(map_storage)? {
            return Err(PromotionLedgerError::new("promotion-ledger-already-exists"));
        }
        let (anchor_observation, anchor_length) = append_anchor_record(&anchor, &record)?;
        let snapshot = authenticate_snapshot(
            ReviewSnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &key,
        )?;
        publish_file(&root, STATE_NAME, &snapshot, 0).map_err(map_storage)?;
        sync_directory(&root).map_err(map_storage)?;
        drop(_guard);
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
            expected_head: snapshot.head_sha256.clone(),
        };
        test_final_validation_pause(&ledger.root_path);
        let _guard = FileLock::exclusive(&ledger.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        ledger.require_published_current(&snapshot)?;
        drop(_guard);
        Ok(ledger)
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
