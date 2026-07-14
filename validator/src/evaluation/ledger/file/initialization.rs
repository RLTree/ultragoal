impl FileEvaluationExecutionLedger {
    pub fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)?;
        ensure_only_known_entries(&root_path, &root, true)?;
        let lock = open_lock(&root, true)?;
        let lock_identity = safe_file_identity(&lock)?;
        let anchor = open_anchor(&root, true)?;
        let anchor_authority = safe_file_identity(&anchor)?.authority();
        let key_id = sha256(&key);
        let core = SnapshotCore {
            schema_version: "EvaluationExecutionLedger-v1".to_owned(),
            generation: 0,
            previous_head_sha256: sha256(b"evaluation-ledger-genesis"),
            key_id: key_id.clone(),
            lock_identity,
            anchor_authority,
            binding: binding.clone(),
            state: EvaluationLedgerState::Initialized,
        };
        let record = authenticate_anchor_record(
            AnchorRecordPayload {
                schema_version: "EvaluationExecutionAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: sha256(ANCHOR_GENESIS),
                core: core.clone(),
            },
            &key,
        )?;
        require_named_lock_identity(&root, lock_identity)?;
        require_named_anchor_authority(&root, anchor_authority)?;
        let _guard = FileLock::exclusive(&lock)?;
        require_named_lock_identity(&root, lock_identity)?;
        require_named_anchor_authority(&root, anchor_authority)?;
        if entry_exists(&root, STATE_NAME)? {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-already-exists",
            ));
        }
        let (anchor_observation, anchor_length) = append_anchor_record(&anchor, &record)?;
        let snapshot = authenticate_snapshot(
            SnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &key,
        )?;
        publish_file(&root, STATE_NAME, &snapshot, 0)?;
        sync_directory(&root)?;
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
        let _guard = FileLock::exclusive(&ledger.lock)?;
        ledger.require_published_current(&snapshot)?;
        drop(_guard);
        Ok(ledger)
    }

    pub fn open(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        let root_path = root.as_ref().to_path_buf();
        let (root, root_identity) = open_safe_directory(&root_path)?;
        let lock = open_lock(&root, false)?;
        let lock_identity = safe_file_identity(&lock)?;
        let anchor = open_anchor(&root, false)?;
        let anchor_authority = safe_file_identity(&anchor)?.authority();
        let key_id = sha256(&key);
        require_named_lock_identity(&root, lock_identity)?;
        require_named_anchor_authority(&root, anchor_authority)?;
        let (pending, current) = {
            let _guard = FileLock::exclusive(&lock)?;
            require_named_lock_identity(&root, lock_identity)?;
            require_named_anchor_authority(&root, anchor_authority)?;
            let pending = ensure_only_known_entries(&root_path, &root, false)?;
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
        let _guard = FileLock::exclusive(&ledger.lock)?;
        ledger.require_unchanged_current(pending.as_ref(), &current)?;
        drop(_guard);
        Ok(ledger)
    }

    /// Read-only authenticated inspection. This does not create lock, temp,
    /// recovery, or audit files.
    pub fn inspect(&self) -> Result<EvaluationLedgerState, EvaluationLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        let state = current.snapshot.payload.core.state.clone();
        test_final_validation_pause(&self.root_path);
        self.require_unchanged_current(pending.as_ref(), &current)?;
        Ok(state)
    }

    pub fn reserve(&mut self) -> Result<(), EvaluationLedgerError> {
        self.transition(|state| match state {
            EvaluationLedgerState::Initialized => Ok(EvaluationLedgerState::Reserved),
            _ => Err(EvaluationLedgerError::new(
                "evaluation-execution-reservation-conflict",
            )),
        })
    }

    pub fn publish_result(
        &mut self,
        run_sha256: impl Into<String>,
        artifact_set_sha256: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        let run_sha256 = run_sha256.into();
        let artifact_set_sha256 = artifact_set_sha256.into();
        if !super::valid_sha256(&run_sha256) || !super::valid_sha256(&artifact_set_sha256) {
            return Err(EvaluationLedgerError::new(
                "evaluation-publication-binding-invalid",
            ));
        }
        self.transition(|state| match state {
            EvaluationLedgerState::Reserved => Ok(EvaluationLedgerState::Published {
                run_sha256,
                artifact_set_sha256,
            }),
            _ => Err(EvaluationLedgerError::new(
                "evaluation-publication-transition-refused",
            )),
        })
    }

    pub fn mark_interrupted(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        let causal_code = checked_causal_code(causal_code)?;
        self.transition(|state| match state {
            EvaluationLedgerState::Reserved | EvaluationLedgerState::Published { .. } => {
                Ok(EvaluationLedgerState::Interrupted { causal_code })
            }
            _ => Err(EvaluationLedgerError::new(
                "evaluation-interruption-transition-refused",
            )),
        })
    }
}
