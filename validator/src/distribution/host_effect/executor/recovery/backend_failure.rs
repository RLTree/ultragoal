impl<'a> SupportedHostEffectExecutor<'a> {
    fn finish_backend_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        mut completed: Vec<CommandCaptureDigest>,
        failure: BackendFailure,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        if failure.capture.stdout.len() <= model::EXACT_OUTPUT_LIMIT_BYTES
            && failure.capture.stderr.len() <= model::EXACT_OUTPUT_LIMIT_BYTES
        {
            if let Ok(digest) = failure.capture.digest(completed.len()) {
                completed.push(digest);
            }
        }
        let state = if failure.started {
            HostEffectState::Ambiguous
        } else {
            HostEffectState::Failed
        };
        let mut originating_error_ids = vec![failure.id];
        let completed_at_unix_ms = match clock.sample() {
            Ok(sample) => sample.unix_ms(),
            Err(_clock_failure) => {
                append_error_cause(&mut originating_error_ids, HostEffectExecutorErrorId::Io);
                0
            }
        };
        let returned_error_id = *originating_error_ids
            .last()
            .expect("backend failure always supplies an originating cause");
        let outcome_sha256 = match outcome_digest(
            effect_identity_sha256,
            effect,
            state,
            &completed,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        ) {
            Ok(outcome_sha256) => outcome_sha256,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        let outcome = match terminal_outcome(
            effect,
            state,
            &completed,
            outcome_sha256.clone(),
            completed_at_unix_ms,
        ) {
            Ok(outcome) => outcome,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        match self.transition_terminal(effect, state, outcome_sha256) {
            Ok(terminal) => Err(self.verified_terminal_failure(
                effect,
                effect_identity_sha256,
                terminal,
                outcome,
                returned_error_id,
                originating_error_ids,
            )),
            Err(transition_failure) => {
                append_error_cause(&mut originating_error_ids, transition_failure.id());
                Err(self.prepublication_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    outcome,
                    originating_error_ids,
                ))
            }
        }
    }

    fn finish_started_failure(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        completed: Vec<CommandCaptureDigest>,
        id: HostEffectExecutorErrorId,
        clock: &mut dyn RootTrustedClock,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        self.finish_backend_failure(
            effect,
            effect_identity_sha256,
            completed,
            BackendFailure {
                id,
                started: true,
                capture: model::CommandCapture::empty_failure(),
            },
            clock,
        )
    }

    fn finish_terminal_with_timestamp(
        &self,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        completed: Vec<CommandCaptureDigest>,
        id: HostEffectExecutorErrorId,
        state: HostEffectState,
        completed_at_unix_ms: u64,
    ) -> Result<HostEffectExecutionReceipt, HostEffectExecutorFailure> {
        let mut originating_error_ids = vec![id];
        let outcome_sha256 = match outcome_digest(
            effect_identity_sha256,
            effect,
            state,
            &completed,
            self.policy.policy_sha256(),
            completed_at_unix_ms,
        ) {
            Ok(outcome_sha256) => outcome_sha256,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        let outcome = match terminal_outcome(
            effect,
            state,
            &completed,
            outcome_sha256.clone(),
            completed_at_unix_ms,
        ) {
            Ok(outcome) => outcome,
            Err(outcome_failure) => {
                append_error_cause(&mut originating_error_ids, outcome_failure.id());
                return Err(self.post_reservation_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    HostEffectExecutorErrorId::RecoveryRequired,
                    originating_error_ids,
                    None,
                ));
            }
        };
        match self.transition_terminal(effect, state, outcome_sha256) {
            Ok(terminal) => Err(self.verified_terminal_failure(
                effect,
                effect_identity_sha256,
                terminal,
                outcome,
                id,
                originating_error_ids,
            )),
            Err(transition_failure) => {
                append_error_cause(&mut originating_error_ids, transition_failure.id());
                Err(self.prepublication_terminal_recovery_failure(
                    effect,
                    effect_identity_sha256,
                    outcome,
                    originating_error_ids,
                ))
            }
        }
    }
}
