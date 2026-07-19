impl FilePromotionReviewLedger {
    pub(crate) fn issue_bound_attestation(
        &mut self,
        binding_sha256: &str,
    ) -> Result<String, PromotionLedgerError> {
        if !super::valid_sha256(binding_sha256) {
            return Err(PromotionLedgerError::new(
                "promotion-attestation-binding-invalid",
            ));
        }
        let attestation = hmac(
            format!(
                "promotion-attestation-v1|{}|{}|{}|{}",
                binding_sha256,
                self.binding.digest(),
                self.binding.reviewer_id,
                self.binding.review_session_id,
            )
            .as_bytes(),
            &self.key,
        )
        .map_err(map_storage)?;
        self.mutate(|state| match state {
            PromotionLedgerState::Ready => Ok((
                PromotionLedgerState::Issued {
                    binding_sha256: binding_sha256.to_owned(),
                    attestation_sha256: attestation.clone(),
                },
                attestation.clone(),
            )),
            PromotionLedgerState::Issued {
                binding_sha256: existing_binding,
                attestation_sha256,
            } if existing_binding == binding_sha256 && attestation_sha256 == attestation => Ok((
                PromotionLedgerState::Issued {
                    binding_sha256: existing_binding,
                    attestation_sha256: attestation_sha256.clone(),
                },
                attestation_sha256,
            )),
            _ => Err(PromotionLedgerError::new(
                "promotion-attestation-issuance-refused",
            )),
        })
    }

    pub(crate) fn consume_attestation(
        &mut self,
        binding_sha256: &str,
        reviewer_id: &str,
        review_id: &str,
        attestation_sha256: &str,
    ) -> Result<PromotionConsumptionOutcome, PromotionLedgerError> {
        let expected_review_id =
            sha256(format!("promotion-review|{binding_sha256}|{attestation_sha256}").as_bytes());
        if reviewer_id != self.binding.reviewer_id
            || !super::valid_sha256(binding_sha256)
            || !super::valid_sha256(review_id)
            || !super::valid_sha256(attestation_sha256)
            || review_id != expected_review_id
        {
            return Ok(PromotionConsumptionOutcome::Refused {
                causal_code: "promotion-review-attestation-invalid",
            });
        }
        self.mutate(|state| match state {
            PromotionLedgerState::Issued {
                binding_sha256: issued_binding,
                attestation_sha256: issued_attestation,
            } if issued_binding == binding_sha256 && issued_attestation == attestation_sha256 => {
                Ok((
                    PromotionLedgerState::Consumed {
                        binding_sha256: issued_binding,
                        review_id: review_id.to_owned(),
                        attestation_sha256: issued_attestation,
                    },
                    PromotionConsumptionOutcome::Consumed,
                ))
            }
            PromotionLedgerState::Consumed {
                binding_sha256: consumed_binding,
                review_id,
                attestation_sha256: consumed_attestation,
            } => {
                let outcome = PromotionConsumptionOutcome::AlreadyConsumed {
                    review_id: review_id.clone(),
                };
                Ok((
                    PromotionLedgerState::Consumed {
                        binding_sha256: consumed_binding,
                        review_id,
                        attestation_sha256: consumed_attestation,
                    },
                    outcome,
                ))
            }
            _ => Ok((
                state,
                PromotionConsumptionOutcome::Refused {
                    causal_code: "promotion-review-consumption-refused",
                },
            )),
        })
    }

    fn mutate<T>(
        &mut self,
        update: impl FnOnce(
            PromotionLedgerState,
        ) -> Result<(PromotionLedgerState, T), PromotionLedgerError>,
    ) -> Result<T, PromotionLedgerError> {
        self.validate_descriptors()?;
        let _guard = FileLock::exclusive(&self.lock)
            .map_err(|_| PromotionLedgerError::new("promotion-ledger-lock-failed"))?;
        self.validate_descriptors()?;
        let (pending, current) = self.read_locked_current()?;
        let repaired = pending.is_some() || current.partial_tail_from.is_some();
        if let Some(pending) = pending {
            remove_pending_publication(&self.root, &pending)?;
            sync_directory(&self.root).map_err(map_storage)?;
        }
        if let Some(length) = current.partial_tail_from {
            self.anchor
                .set_len(length)
                .map_err(|_| PromotionLedgerError::new("promotion-anchor-repair-failed"))?;
            self.anchor
                .sync_all()
                .map_err(|_| PromotionLedgerError::new("promotion-anchor-fsync-failed"))?;
        }
        if current.snapshot.head_sha256 != self.expected_head {
            self.expected_head = current.snapshot.head_sha256.clone();
        }
        let (next_state, result) = update(current.snapshot.payload.core.state.clone())?;
        if next_state == current.snapshot.payload.core.state && !repaired {
            test_final_validation_pause(&self.root_path);
            self.require_unchanged_current(None, &current)?;
            return Ok(result);
        }
        let current = current.snapshot;
        let generation = current
            .payload
            .core
            .generation
            .checked_add(1)
            .ok_or_else(|| PromotionLedgerError::new("promotion-ledger-generation-overflow"))?;
        let core = ReviewSnapshotCore {
            schema_version: "PromotionReviewLedger-v1".to_owned(),
            generation,
            previous_head_sha256: current.head_sha256,
            key_id: self.key_id.clone(),
            lock_identity: self.lock_identity,
            anchor_authority: self.anchor_authority,
            binding: self.binding.clone(),
            state: next_state,
        };
        let record = authenticate_anchor_record(
            ReviewAnchorRecordPayload {
                schema_version: "PromotionReviewAnchorRecord-v1".to_owned(),
                prior_anchor_head_sha256: current.payload.anchor_head_sha256,
                core: core.clone(),
            },
            &self.key,
        )?;
        let (anchor_observation, anchor_length) = append_anchor_record(&self.anchor, &record)?;
        let next = authenticate_snapshot(
            ReviewSnapshotPayload {
                core,
                anchor_observation,
                anchor_length,
                anchor_head_sha256: record.head_sha256,
            },
            &self.key,
        )?;
        test_publication_pause(&self.root_path);
        publish_file(&self.root, STATE_NAME, &next, generation).map_err(map_storage)?;
        sync_directory(&self.root).map_err(map_storage)?;
        test_final_validation_pause(&self.root_path);
        self.require_published_current(&next)?;
        self.expected_head = next.head_sha256;
        Ok(result)
    }

    fn read_locked_current(
        &self,
    ) -> Result<(Option<PendingReviewPublication>, CurrentReviewSnapshot), PromotionLedgerError>
    {
        let pending = ensure_entries(&self.root_path, &self.root, false)?;
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
}
