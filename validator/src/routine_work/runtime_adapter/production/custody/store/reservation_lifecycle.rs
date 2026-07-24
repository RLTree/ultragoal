use super::*;

impl FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn reserve(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
    ) -> Result<DurableWrite<u64>, RoutineError> {
        validate_token(token)?;
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "reserve", None),
            |payload, tick, _head| {
                if payload.consumed_grants.contains(&token.grant_id)
                    || payload
                        .attempts
                        .values()
                        .any(|record| record.binding == token.binding && record.state.pending())
                {
                    return Err(error("routine-production-reservation-conflict"));
                }
                if let Some(protocol) = payload.effects.get(&token.binding.effect_id)
                    && protocol != &token.binding.protocol_id
                {
                    return Err(error("routine-production-effect-protocol-conflict"));
                }
                validate_reservation_capacity(payload, token)?;
                let expires_tick = tick
                    .checked_add(GRANT_TTL_SECONDS)
                    .ok_or_else(|| error("routine-production-time-overflow"))?;
                let recovery_deadline_tick = tick
                    .checked_add(RECOVERY_TTL_SECONDS)
                    .ok_or_else(|| error("routine-production-time-overflow"))?;
                let record = ProtocolRecord {
                    binding: token.binding.clone(),
                    request_id: token.request_id.clone(),
                    grant_id: token.grant_id.clone(),
                    recovery_marker: token.recovery_marker.clone(),
                    predecessor_continuations: token.predecessor_continuations.clone(),
                    state: AttemptState::Reserved,
                    owner: token.owner.clone(),
                    child: None,
                    launch_stage: None,
                    intents: token.intents.clone(),
                    next_intent: 0,
                    issued_tick: tick,
                    expires_tick,
                    recovery_deadline_tick,
                    output_journal: token.output_journal.clone(),
                    terminal: None,
                    publication_ambiguity: None,
                    failure_evidence: Vec::new(),
                };
                payload.consumed_grants.insert(token.grant_id.clone());
                payload.effects.insert(
                    token.binding.effect_id.clone(),
                    token.binding.protocol_id.clone(),
                );
                payload.attempts.insert(token.grant_id.clone(), record);
                Ok((expires_tick, true))
            },
        )
    }

    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn validate_reserved(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
    ) -> Result<(), RoutineError> {
        self.with_payload(local, false, |payload, tick| {
            let record = exact_record(payload, token)?;
            if record.state != AttemptState::Reserved || tick > record.expires_tick {
                return Err(error("routine-production-grant-not-reserved"));
            }
            Ok(())
        })
    }

    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn prepare_spawn(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        child: &ChildLease,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "child-start", None),
            |payload, tick, _head| {
                let record = exact_record_mut(payload, token)?;
                if tick > record.expires_tick {
                    return Err(error("routine-production-spawn-authority-invalid"));
                }
                match record.state {
                    AttemptState::Staged
                        if record.child.is_none()
                            && record.launch_stage.as_ref().map(|stage| &stage.intent)
                                == Some(&child.intent)
                            && record.intents.get(record.next_intent) == Some(&child.intent) =>
                    {
                        record.child = Some(child.clone());
                        record.state = AttemptState::Started;
                        Ok(((), true))
                    }
                    AttemptState::Started if record.child.as_ref() == Some(child) => {
                        Ok(((), false))
                    }
                    _ => Err(error("routine-production-spawn-authority-invalid")),
                }
            },
        )
    }

    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn record_launch_stage(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        stage: &LaunchStageRecord,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "launch-stage", None),
            |payload, _tick, _head| {
                let record = exact_record_mut(payload, token)?;
                match (&record.launch_stage, record.child.as_ref()) {
                    (None, None)
                        if matches!(
                            record.state,
                            AttemptState::Reserved | AttemptState::Started
                        ) =>
                    {
                        if record.intents.get(record.next_intent) != Some(&stage.intent) {
                            return Err(error("routine-production-launch-intent-order-invalid"));
                        }
                        record.launch_stage = Some(stage.clone());
                        record.state = AttemptState::Staged;
                        Ok(((), true))
                    }
                    (Some(current), None) if current == stage => Ok(((), false)),
                    _ => Err(error("routine-production-launch-stage-transition-invalid")),
                }
            },
        )
    }

    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn record_launch_cleaned(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        stage: &LaunchStageRecord,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "launch-cleanup", None),
            |payload, _tick, _head| {
                let record = exact_record_mut(payload, token)?;
                if record.child.is_some() || record.launch_stage.as_ref() != Some(stage) {
                    return Err(error(
                        "routine-production-launch-cleanup-transition-invalid",
                    ));
                }
                record.launch_stage = None;
                record.state = AttemptState::Started;
                record.next_intent = record
                    .next_intent
                    .checked_add(1)
                    .ok_or_else(|| error("routine-production-intent-order-overflow"))?;
                Ok(((), true))
            },
        )
    }

    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn record_process_reaped(
        &self,
        local: &mut LocalHead,
        token: &ReservationToken,
        child: &ChildLease,
    ) -> Result<DurableWrite<()>, RoutineError> {
        self.transition_payload(
            local,
            PublicationContext::write(&token.grant_id, "child-reaped", None),
            |payload, _tick, _head| {
                let record = exact_record_mut(payload, token)?;
                if record.state != AttemptState::Started
                    || record.child.as_ref() != Some(child)
                    || record.intents.get(record.next_intent) != Some(&child.intent)
                {
                    return Err(error("routine-production-child-reap-binding-invalid"));
                }
                record.child = None;
                Ok(((), true))
            },
        )
    }
}
