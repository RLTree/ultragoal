impl<'a> SupportedHostEffectExecutor<'a> {
    fn reobserve_post_reservation(
        &self,
        effect: &AuthorizedHostEffect,
    ) -> PostReservationLedgerEvidence {
        const MAX_STABLE_OBSERVATION_ATTEMPTS: usize = 3;
        let mut observation_failed = false;
        for _ in 0..MAX_STABLE_OBSERVATION_ATTEMPTS {
            let head_before = match self.ledger.head() {
                Ok(head) => head,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            let record_before = match self.ledger.read(effect.permit().permit_id()) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            let record_after = match self.ledger.read(effect.permit().permit_id()) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            let head_after = match self.ledger.head() {
                Ok(head) => head,
                Err(_ledger_observation_failure) => {
                    observation_failed = true;
                    continue;
                }
            };
            if head_before != head_after || record_before != record_after {
                continue;
            }
            if head_after != *record_after.current_head()
                || record_after.reservation().permit_id() != effect.permit().permit_id()
            {
                continue;
            }
            if record_after == *effect.record() && head_after == *record_after.current_head() {
                return PostReservationLedgerEvidence {
                    ledger_head: head_after,
                    ledger_record: record_after,
                    exact_current_observation: true,
                    classification: HostEffectPostReservationLedgerClassification::StillInFlight,
                    terminal_state: None,
                    observation_error_id: None,
                };
            }
            if record_after.reservation() == effect.record().reservation()
                && head_after == *record_after.current_head()
                && matches!(
                    record_after.state(),
                    HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
                )
            {
                let terminal_state = Some(record_after.state());
                return PostReservationLedgerEvidence {
                    ledger_head: head_after,
                    ledger_record: record_after,
                    exact_current_observation: true,
                    classification: HostEffectPostReservationLedgerClassification::TerminalObserved,
                    terminal_state,
                    observation_error_id: None,
                };
            }
            return PostReservationLedgerEvidence {
                ledger_head: head_after,
                ledger_record: record_after,
                exact_current_observation: true,
                classification: HostEffectPostReservationLedgerClassification::ObservationRejected,
                terminal_state: None,
                observation_error_id: None,
            };
        }
        PostReservationLedgerEvidence {
            ledger_head: effect.record().current_head().clone(),
            ledger_record: effect.record().clone(),
            exact_current_observation: false,
            classification: HostEffectPostReservationLedgerClassification::ObservationUnavailable,
            terminal_state: None,
            observation_error_id: observation_failed
                .then_some(HostEffectExecutorErrorId::LedgerSubstitution),
        }
    }

    #[cfg(test)]
    pub(super) fn execute_authorized_for_test(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        target_lease: &mut dyn HostTargetLease,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        self.execute_authorized(capability, effect, target_lease, clock, cancellation)
    }
}

fn receipt_name(permit_id: &str) -> Result<String, HostEffectExecutorFailure> {
    let digest = permit_id
        .strip_prefix("sha256:")
        .ok_or_else(|| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Replay))?;
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HostEffectExecutorFailure::new(
            HostEffectExecutorErrorId::Replay,
        ));
    }
    Ok(format!("effect-{digest}.json"))
}

fn fallback_effect_identity(effect: &AuthorizedHostEffect) -> String {
    model::digest_bytes(
        format!(
            "harness-ultragoal.unavailable-effect-identity.v1\n{}\n{}",
            effect.permit().permit_id(),
            effect.record().current_head().head_sha256(),
        )
        .as_bytes(),
    )
}

fn append_error_cause(
    originating_error_ids: &mut Vec<HostEffectExecutorErrorId>,
    error_id: HostEffectExecutorErrorId,
) {
    if error_id != HostEffectExecutorErrorId::RecoveryRequired
        && !originating_error_ids.contains(&error_id)
    {
        originating_error_ids.push(error_id);
    }
}

fn terminal_outcome(
    effect: &AuthorizedHostEffect,
    state: HostEffectState,
    command_digests: &[CommandCaptureDigest],
    outcome_sha256: String,
    completed_at_unix_ms: u64,
) -> Result<HostEffectOutcome, HostEffectExecutorFailure> {
    HostEffectOutcome::new(
        effect.permit().permit_id().to_owned(),
        state,
        command_digests
            .iter()
            .map(|capture| capture.combined_sha256.clone())
            .collect(),
        None,
        outcome_sha256,
        completed_at_unix_ms,
    )
    .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))
}

fn outcome_digest(
    effect_identity_sha256: &str,
    effect: &AuthorizedHostEffect,
    state: HostEffectState,
    command_digests: &[CommandCaptureDigest],
    policy_sha256: &str,
    completed_at_unix_ms: u64,
) -> Result<String, HostEffectExecutorFailure> {
    #[derive(Serialize)]
    struct Outcome<'a> {
        schema: &'static str,
        effect_identity_sha256: &'a str,
        permit_id: &'a str,
        state: HostEffectState,
        command_digests: &'a [CommandCaptureDigest],
        policy_sha256: &'a str,
        completed_at_unix_ms: u64,
    }
    digest_json(&Outcome {
        schema: "harness-ultragoal.host-effect-outcome.v1",
        effect_identity_sha256,
        permit_id: effect.permit().permit_id(),
        state,
        command_digests,
        policy_sha256,
        completed_at_unix_ms,
    })
}
