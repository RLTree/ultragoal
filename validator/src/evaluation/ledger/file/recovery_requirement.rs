impl FileEvaluationExecutionLedger {
    fn require_recovery(
        &mut self,
        causal_code: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        let causal_code = checked_causal_code(causal_code)?;
        self.transition(None, |_, current| {
            match current.payload.core.state.clone() {
                EvaluationLedgerState::Reserved
                | EvaluationLedgerState::Published { .. }
                | EvaluationLedgerState::Interrupted { .. } => {
                    Ok(EvaluationLedgerState::RecoveryRequired { causal_code })
                }
                _ => Err(EvaluationLedgerError::new(
                    "evaluation-recovery-transition-refused",
                )),
            }
        })
    }

    /// Recovery may only restore a publication that the same authenticated
    /// reservation already recorded. Absence of a recorded publication proves
    /// neither that the child stopped nor that staged custody was removed.
    fn reconcile_authenticated_publication(&mut self) -> Result<(), EvaluationLedgerError> {
        self.transition(None, |ledger, current| match &current.payload.core.state {
            EvaluationLedgerState::RecoveryRequired { .. } => {
                let (run_sha256, artifact_set_sha256) =
                    ledger.authenticated_recovery_publication(current)?;
                Ok(EvaluationLedgerState::Published {
                    run_sha256,
                    artifact_set_sha256,
                })
            }
            _ => Err(EvaluationLedgerError::new(
                "evaluation-recovery-not-required",
            )),
        })
    }

    /// This stays inside the custody owner: callers cannot supply a run or
    /// artifact digest and thereby reinterpret an ambiguous effect as a
    /// publication. The journal scanner already verifies the MAC, full
    /// execution binding, and reservation identity for every returned record.
    fn authenticated_recovery_publication(
        &self,
        current: &AuthenticatedSnapshot,
    ) -> Result<(String, String), EvaluationLedgerError> {
        let reservation_id = current
            .payload
            .core
            .reservation_id_sha256
            .as_deref()
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-recovery-binding-invalid"))?;
        let before = safe_file_identity(&self.anchor)?;
        if before != current.payload.anchor_observation {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-changed-during-recovery",
            ));
        }
        let bytes = read_file_bytes(&self.anchor, MAX_ANCHOR_JOURNAL_BYTES)?;
        let after = safe_file_identity(&self.anchor)?;
        if after != before {
            return Err(EvaluationLedgerError::new(
                "evaluation-anchor-changed-during-recovery",
            ));
        }
        let scan = scan_anchor_journal(
            &bytes,
            &self.key,
            &self.key_id,
            self.lock_identity,
            self.anchor_authority,
            &self.binding,
        )?;
        let current_index = scan
            .records
            .iter()
            .position(|record| {
                record.end == current.payload.anchor_length
                    && record.record.head_sha256 == current.payload.anchor_head_sha256
                    && record.record.payload.core == current.payload.core
            })
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-recovery-journal-missing"))?;
        scan.records[..current_index]
            .iter()
            .rev()
            .find_map(|record| {
                (record.record.payload.core.reservation_id_sha256.as_deref()
                    == Some(reservation_id))
                .then(|| match &record.record.payload.core.state {
                    EvaluationLedgerState::Published {
                        run_sha256,
                        artifact_set_sha256,
                    } => Some((run_sha256.clone(), artifact_set_sha256.clone())),
                    _ => None,
                })
                .flatten()
            })
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-recovery-publication-unproven"))
    }

    fn complete(&mut self) -> Result<(), EvaluationLedgerError> {
        self.transition(None, |_, current| {
            match current.payload.core.state.clone() {
                EvaluationLedgerState::Published {
                    run_sha256,
                    artifact_set_sha256,
                } => Ok(EvaluationLedgerState::Terminal {
                    run_sha256,
                    artifact_set_sha256,
                }),
                _ => Err(EvaluationLedgerError::new(
                    "evaluation-terminal-transition-refused",
                )),
            }
        })
    }

    pub(crate) fn terminal_proof(&self) -> Result<ExecutionTerminalProof, EvaluationLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        test_final_validation_pause(&self.root_path);
        self.require_unchanged_current(pending.as_ref(), &current)?;
        let snapshot = current.snapshot;
        let EvaluationLedgerState::Terminal {
            run_sha256,
            artifact_set_sha256,
        } = snapshot.payload.core.state
        else {
            return Err(EvaluationLedgerError::new(
                "evaluation-terminal-proof-unavailable",
            ));
        };
        Ok(ExecutionTerminalProof {
            binding: self.binding.clone(),
            run_sha256,
            artifact_set_sha256,
            ledger_head_sha256: snapshot.head_sha256,
        })
    }

    fn transition(
        &mut self,
        reservation_id_sha256: Option<String>,
        update: impl FnOnce(
            &Self,
            &AuthenticatedSnapshot,
        ) -> Result<EvaluationLedgerState, EvaluationLedgerError>,
    ) -> Result<(), EvaluationLedgerError> {
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
        if current.head_sha256 != self.expected_head {
            self.expected_head = current.head_sha256.clone();
        }
        let next_state = update(self, &current)?;
        let next_generation = current
            .payload
            .core
            .generation
            .checked_add(1)
            .ok_or_else(|| EvaluationLedgerError::new("evaluation-ledger-generation-overflow"))?;
        let core = SnapshotCore {
            schema_version: "EvaluationExecutionLedger-v1".to_owned(),
            generation: next_generation,
            previous_head_sha256: current.head_sha256,
            key_id: self.key_id.clone(),
            lock_identity: self.lock_identity,
            anchor_authority: self.anchor_authority,
            binding: self.binding.clone(),
            reservation_id_sha256: reservation_id_sha256
                .or_else(|| current.payload.core.reservation_id_sha256.clone()),
            state: next_state,
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
        test_publication_pause(&self.root_path);
        publish_file(&self.root, STATE_NAME, &next, next_generation)?;
        sync_directory(&self.root)?;
        test_final_validation_pause(&self.root_path);
        self.require_published_current(&next)?;
        self.expected_head = next.head_sha256;
        Ok(())
    }

    fn read_locked_current(
        &self,
    ) -> Result<(Option<PendingPublication>, CurrentSnapshot), EvaluationLedgerError> {
        let pending = ensure_only_known_entries(&self.root_path, &self.root, false)?;
        let current = read_current(
            &self.root,
            &self.anchor,
            self.lock_identity,
            self.anchor_authority,
            &self.key,
            &self.key_id,
            &self.binding,
        )?;
        validate_pending_generation(pending.as_ref(), &current.snapshot)?;
        Ok((pending, current))
    }

    fn require_unchanged_current(
        &self,
        expected_pending: Option<&PendingPublication>,
        expected: &CurrentSnapshot,
    ) -> Result<(), EvaluationLedgerError> {
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        if pending.as_ref() != expected_pending
            || current.snapshot.head_sha256 != expected.snapshot.head_sha256
            || current.observed_state_head_sha256 != expected.observed_state_head_sha256
            || current.observed_anchor != expected.observed_anchor
            || current.partial_tail_from != expected.partial_tail_from
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-final-current-changed",
            ));
        }
        self.validate_descriptors()
    }
}
