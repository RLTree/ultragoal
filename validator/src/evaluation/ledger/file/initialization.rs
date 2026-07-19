impl FileEvaluationExecutionLedger {
    pub fn initialize(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, EvaluationLedgerError> {
        initialize_execution_ledger(root.as_ref().to_path_buf(), key, binding)
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
        let reservation_id_sha256 =
            sha256(&serde_json::to_vec(&self.binding).map_err(|_| {
                EvaluationLedgerError::new("evaluation-ledger-serialization-failed")
            })?);
        self.transition(Some(reservation_id_sha256), |state| match state {
            EvaluationLedgerState::Initialized => Ok(EvaluationLedgerState::Reserved),
            _ => Err(EvaluationLedgerError::new(
                "evaluation-execution-reservation-conflict",
            )),
        })
    }

    pub fn reserve_outcome(
        &mut self,
        reservation_id: &str,
    ) -> Result<ExecutionReservationOutcome, EvaluationLedgerError> {
        if !super::valid_sha256(reservation_id) {
            return Err(EvaluationLedgerError::new(
                "evaluation-execution-reservation-id-invalid",
            ));
        }
        self.reserve_outcome_locked(reservation_id)
    }

    /// The decision and write share one file lock. A loser therefore receives
    /// the authenticated state that caused its loss, not a later observation.
    fn reserve_outcome_locked(
        &self,
        reservation_id: &str,
    ) -> Result<ExecutionReservationOutcome, EvaluationLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if let Some(pending) = pending {
            remove_pending_publication(&self.root, &pending)?;
            sync_directory(&self.root)?;
        }
        if let Some(length) = current.partial_tail_from {
            self.anchor
                .set_len(length)
                .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-repair-failed"))?;
            self.anchor
                .sync_all()
                .map_err(|_| EvaluationLedgerError::new("evaluation-anchor-fsync-failed"))?;
        }
        let current = current.snapshot;
        if !matches!(
            current.payload.core.state,
            EvaluationLedgerState::Initialized
        ) {
            let existing_reservation_id = current.payload.core.reservation_id_sha256.clone();
            return Ok(match current.payload.core.state {
                EvaluationLedgerState::Reserved
                    if existing_reservation_id.as_deref() == Some(reservation_id) =>
                {
                    ExecutionReservationOutcome::AlreadyReserved
                }
                EvaluationLedgerState::Reserved => ExecutionReservationOutcome::Lost {
                    causal_code: "evaluation-execution-reservation-conflict",
                },
                EvaluationLedgerState::Published {
                    run_sha256,
                    artifact_set_sha256,
                } => ExecutionReservationOutcome::AlreadyPublished {
                    run_sha256,
                    artifact_set_sha256,
                },
                EvaluationLedgerState::Interrupted { causal_code } => {
                    ExecutionReservationOutcome::Interrupted { causal_code }
                }
                EvaluationLedgerState::RecoveryRequired { causal_code } => {
                    ExecutionReservationOutcome::RecoveryRequired { causal_code }
                }
                EvaluationLedgerState::Terminal {
                    run_sha256,
                    artifact_set_sha256,
                } => ExecutionReservationOutcome::Terminal {
                    run_sha256,
                    artifact_set_sha256,
                },
                EvaluationLedgerState::Initialized => {
                    return Err(EvaluationLedgerError::new(
                        "evaluation-reservation-state-raced",
                    ));
                }
            });
        }
        let generation = current
            .payload
            .core
            .generation
            .checked_add(1)
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-ledger-generation-overflow"))?;
        let core = SnapshotCore {
            schema_version: "EvaluationExecutionLedger-v1".to_owned(),
            generation,
            previous_head_sha256: current.head_sha256,
            key_id: self.key_id.clone(),
            lock_identity: self.lock_identity,
            anchor_authority: self.anchor_authority,
            binding: self.binding.clone(),
            reservation_id_sha256: Some(reservation_id.to_owned()),
            state: EvaluationLedgerState::Reserved,
        };
        let record = authenticate_anchor_record(
            AnchorRecordPayload {
                schema_version: "EvaluationExecutionAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: current.payload.anchor_head_sha256,
                core: core.clone(),
            },
            &self.key,
        )?;
        let (anchor_observation, anchor_length) = append_anchor_record(&self.anchor, &record)?;
        let next = authenticate_snapshot(
            SnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &self.key,
        )?;
        publish_file(&self.root, STATE_NAME, &next, generation)?;
        sync_directory(&self.root)?;
        self.require_published_current(&next)?;
        Ok(ExecutionReservationOutcome::Acquired)
    }

    pub(crate) fn binding(&self) -> &EvaluationExecutionBinding {
        &self.binding
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
        self.transition(None, |state| match state {
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
        self.transition(None, |state| match state {
            EvaluationLedgerState::Reserved | EvaluationLedgerState::Published { .. } => {
                Ok(EvaluationLedgerState::Interrupted { causal_code })
            }
            _ => Err(EvaluationLedgerError::new(
                "evaluation-interruption-transition-refused",
            )),
        })
    }
}
