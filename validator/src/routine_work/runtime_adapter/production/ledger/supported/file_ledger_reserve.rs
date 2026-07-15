use super::*;

impl FileLedger {
    pub(crate) fn reserve(&self, spec: ReservationSpec) -> Result<ReservationToken, RoutineError> {
        validate_spec(&spec)?;
        self.with_payload(true, |payload, tick| {
            if payload.consumed_grants.contains(&spec.grant_id) {
                return Err(error("routine-production-grant-replayed"));
            }
            let existing = payload.protocols.get(&spec.binding.protocol_id).cloned();
            match (&spec.recovery_for, existing.as_ref()) {
                (Some(marker), Some(record))
                    if record.state.pending()
                        && record.recovery_marker == *marker
                        && record.binding == spec.binding
                        && tick <= record.recovery_deadline_tick => {}
                (Some(_), _) => {
                    return Err(error("routine-production-recovery-authority-invalid"));
                }
                (None, Some(record))
                    if spec.reuse_only
                        && record.state == AttemptState::Complete
                        && record.binding == spec.binding
                        && !record.artifacts.is_empty() => {}
                (None, Some(record))
                    if !spec.reuse_only
                        && record.state == AttemptState::Complete
                        && record.binding == spec.binding => {}
                (None, Some(record))
                    if !spec.reuse_only
                        && matches!(
                            record.state,
                            AttemptState::Failed
                                | AttemptState::Cancelled
                                | AttemptState::Incomplete
                        )
                        && record.binding == spec.binding => {}
                (None, Some(_)) => {
                    return Err(error("routine-production-semantic-effect-replayed"));
                }
                (None, None) if !spec.reuse_only => {
                    if payload.effects.contains_key(&spec.binding.effect_id) {
                        return Err(error("routine-production-semantic-effect-replayed"));
                    }
                }
                (None, None) => {
                    return Err(error("routine-production-reuse-without-complete-effect"));
                }
            }
            if let Some(protocol) = payload.effects.get(&spec.binding.effect_id)
                && protocol != &spec.binding.protocol_id
            {
                return Err(error("routine-production-effect-protocol-conflict"));
            }
            validate_reservation_capacity(payload, &spec)?;
            if spec.reuse_only {
                validate_reuse_preauthorization(
                    payload,
                    &spec,
                    spec.reuse_preauthorization
                        .as_ref()
                        .ok_or_else(|| error("routine-production-reservation-spec-invalid"))?,
                    &self.authority_id,
                )?;
            }
            let expires_tick = tick
                .checked_add(GRANT_TTL_SECONDS)
                .ok_or_else(|| error("routine-production-time-overflow"))?;
            let recovery_deadline_tick = tick
                .checked_add(RECOVERY_TTL_SECONDS)
                .ok_or_else(|| error("routine-production-time-overflow"))?;
            payload.consumed_grants.insert(spec.grant_id.clone());
            if spec.reuse_only {
                // A reuse grant is an authorization to read and authenticate
                // the existing Complete record. It must never replace that
                // record with a transient state that malformed or forged
                // external input could later settle as Failed.
                return Ok(ReservationToken {
                    binding: spec.binding,
                    request_id: spec.request_id,
                    grant_id: spec.grant_id,
                    recovery_marker: spec.recovery_marker,
                    recovery_for: spec.recovery_for,
                    reuse_only: true,
                    expires_tick,
                    output_journal: spec.output_journal,
                });
            }
            let artifacts = existing
                .as_ref()
                .filter(|record| record.state.pending())
                .map(|record| record.artifacts.clone())
                .unwrap_or_default();
            let output_journal = existing
                .filter(|record| record.state.pending())
                .map(|record| record.output_journal)
                .unwrap_or(spec.output_journal);
            let record = ProtocolRecord {
                binding: spec.binding.clone(),
                request_id: spec.request_id.clone(),
                grant_id: spec.grant_id.clone(),
                recovery_marker: spec.recovery_marker.clone(),
                recovery_for: spec.recovery_for.clone(),
                state: AttemptState::Reserved,
                reuse_only: spec.reuse_only,
                issued_tick: tick,
                expires_tick,
                recovery_deadline_tick,
                artifacts,
                output_journal: output_journal.clone(),
            };
            payload.effects.insert(
                spec.binding.effect_id.clone(),
                spec.binding.protocol_id.clone(),
            );
            payload
                .protocols
                .insert(spec.binding.protocol_id.clone(), record);
            Ok(ReservationToken {
                binding: spec.binding,
                request_id: spec.request_id,
                grant_id: spec.grant_id,
                recovery_marker: spec.recovery_marker,
                recovery_for: spec.recovery_for,
                reuse_only: false,
                expires_tick,
                output_journal,
            })
        })
    }
    pub(crate) fn validate_reserved(&self, token: &ReservationToken) -> Result<(), RoutineError> {
        self.with_payload(false, |payload, tick| {
            if token.reuse_only {
                exact_completed_reuse_record(payload, token)?;
                if tick > token.expires_tick {
                    return Err(error("routine-production-grant-not-reserved"));
                }
                return Ok(());
            }
            let record = exact_record(payload, token)?;
            if record.state != AttemptState::Reserved || tick > record.expires_tick {
                return Err(error("routine-production-grant-not-reserved"));
            }
            Ok(())
        })
    }
    pub(crate) fn prepare_spawn(&self, token: &ReservationToken) -> Result<(), RoutineError> {
        if token.reuse_only {
            return Err(error("routine-production-spawn-authority-invalid"));
        }
        self.with_payload_conditional(|payload, tick| {
            let record = exact_record_mut(payload, token)?;
            if record.reuse_only || tick > record.expires_tick {
                return Err(error("routine-production-spawn-authority-invalid"));
            }
            match record.state {
                AttemptState::Reserved => {
                    record.state = AttemptState::Started;
                    Ok(((), true))
                }
                // One production reservation covers the dependency-closed
                // intent batch. Later intents may consume only the exact
                // same still-live token; they must not create another
                // durable transition or generation.
                AttemptState::Started => Ok(((), false)),
                AttemptState::Complete
                | AttemptState::Failed
                | AttemptState::Cancelled
                | AttemptState::Incomplete => {
                    Err(error("routine-production-spawn-authority-invalid"))
                }
            }
        })
    }
}
