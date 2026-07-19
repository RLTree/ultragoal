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
                let (run_sha256, artifact_set_sha256, terminal_result) =
                    ledger.authenticated_recovery_publication(current)?;
                Ok(EvaluationLedgerState::Published {
                    run_sha256,
                    artifact_set_sha256,
                    terminal_result,
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
    ) -> Result<(String, String, Vec<u8>), EvaluationLedgerError> {
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
                        terminal_result,
                    } => Some((
                        run_sha256.clone(),
                        artifact_set_sha256.clone(),
                        terminal_result.clone(),
                    )),
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
                    terminal_result,
                } => Ok(EvaluationLedgerState::Terminal {
                    run_sha256,
                    artifact_set_sha256,
                    terminal_result,
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
            ..
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
}
