impl HostLifecycleRecoveryCarrier {
    pub(super) fn reobserve_and_resolve(mut self) -> HostLifecycleTransactionOutcome {
        if self.attempt.observation_attempt >= MAX_REOBSERVATION_ATTEMPTS {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        }
        self.attempt.observation_attempt += 1;
        let Some(recovery) = self.recovery.clone() else {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        };
        let Some((expected_head, expected_record)) = recovery.exact_ledger_observation() else {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        };
        if !self.handoff.revalidate_recovery(
            &self.custody,
            &recovery,
            &self.attempt.effect_identity_sha256,
            &self.attempt.command_plan_sha256,
            &self.attempt.executable_identity_sha256,
            &self.attempt.target_identity_sha256,
        ) || !self.observation_target.revalidate_for_recovery()
        {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        }
        let current = self
            .ledger
            .head()
            .ok()
            .zip(self.ledger.read(&self.attempt.permit_id).ok().flatten());
        let Some((current_head, current_record)) = current else {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        };
        if current_head != *expected_head || current_record != *expected_record {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        }
        match recovery.disposition_state() {
            Some(HostEffectState::Failed) => {
                let cause = self.cause.message();
                let disposition = match self
                    .handoff
                    .issue_recovery_disposition(&mut self.custody, &recovery)
                {
                    Ok(disposition) => disposition,
                    Err(_) => return HostLifecycleTransactionOutcome::RecoveryRequired(self),
                };
                match self.handoff.finalize_recovery(disposition, &recovery) {
                    Ok(()) => HostLifecycleTransactionOutcome::FinalizedFailure(cause),
                    Err(_) => HostLifecycleTransactionOutcome::FinalizedFailure(
                        "host lifecycle recovery finalization failed",
                    ),
                }
            }
            Some(HostEffectState::Settled) => self.reobserve_settled(),
            Some(HostEffectState::Ambiguous)
            | Some(HostEffectState::Reserved)
            | Some(HostEffectState::InFlight)
            | None => HostLifecycleTransactionOutcome::RecoveryRequired(self),
        }
    }

    fn reobserve_settled(mut self) -> HostLifecycleTransactionOutcome {
        let capability = match transaction_policy::current_capability() {
            Ok(capability) => capability,
            Err(_) => return HostLifecycleTransactionOutcome::RecoveryRequired(self),
        };
        let package = self.custody.pre_effect_record().package().clone();
        let cancellation = HostEffectCancellation::default();
        let mut backend = NativeRetainedDescriptorProcessBackend;
        let result = match self.handoff.with_revalidated_observation(|executable| {
            observe(
                &package,
                self.custody.plan(),
                &self.observation_input,
                self.custody.pre_effect_record().expected_observations(),
                executable,
                &capability,
                &mut backend,
                &self.policy,
                &cancellation,
                self.observation_target.cwd_fd(),
                &self.target_root,
                &self.environment,
                self.command_output_sha256.clone(),
            )
        }) {
            Ok(Ok(result)) => result,
            Ok(Err(_)) | Err(_) => {
                return HostLifecycleTransactionOutcome::RecoveryRequired(self);
            }
        };
        let ambiguous = result.completed_effects.len() != self.custody.plan().effects.len();
        let observed = result.observed.clone();
        let completion = if ambiguous {
            HostEffectCompletion::ambiguous(
                self.custody.completion_binding(),
                observed.clone(),
                result.completed_effects,
                result.observations,
                result.effect_cursor,
            )
        } else {
            HostEffectCompletion::settled(
                self.custody.completion_binding(),
                observed.clone(),
                result.completed_effects,
                result.observations,
                result.effect_cursor,
            )
        };
        let recovery_pending = self.custody.recovery_pending();
        if recovery_pending {
            if !ambiguous {
                return HostLifecycleTransactionOutcome::RecoveryRequired(self);
            }
        } else if self.custody.settle(completion).is_err() {
            return HostLifecycleTransactionOutcome::RecoveryRequired(self);
        }
        if ambiguous {
            let mut adapter = ObservedRecoveryAdapter { observed };
            let observed_for_recovery = adapter.observed.clone();
            if self
                .custody
                .recover(&observed_for_recovery, &mut adapter)
                .is_err()
            {
                return HostLifecycleTransactionOutcome::RecoveryRequired(self);
            }
        }
        let finalization = match self.custody.finalization_token() {
            Ok(finalization) => finalization,
            Err(_) => return HostLifecycleTransactionOutcome::RecoveryRequired(self),
        };
        match self.handoff.finalize(finalization) {
            Ok(()) => HostLifecycleTransactionOutcome::Completed(
                super::transaction::HostLifecycleTransactionResult {
                    surfaces: result.surfaces,
                },
            ),
            Err(_) => HostLifecycleTransactionOutcome::FinalizedFailure(
                "host lifecycle terminal finalization failed",
            ),
        }
    }
}

impl HostLifecycleRecoveryCause {
    fn message(self) -> &'static str {
        match self {
            Self::Execution(message)
            | Self::Observation(message)
            | Self::Settlement(message)
            | Self::Finalization(message) => message,
        }
    }
}
