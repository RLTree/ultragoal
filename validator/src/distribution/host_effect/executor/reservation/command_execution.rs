impl<'a> SupportedHostEffectExecutor<'a> {
    fn execute_reserved_commands(
        &mut self,
        capability: &DescriptorExecutionCapability,
        effect: &AuthorizedHostEffect,
        effect_identity_sha256: &str,
        clock: &mut dyn RootTrustedClock,
        cancellation: &HostEffectCancellation,
    ) -> Result<ReservedExecution<Vec<CommandCaptureDigest>>, HostEffectExecutorFailure> {
        let mut command_digests = Vec::with_capacity(effect.plan().commands().len());
        for (command_index, command) in effect.plan().commands().iter().enumerate() {
            if cancellation.is_cancelled() {
                return terminal_execution(self.finish_backend_failure(
                    effect,
                    effect_identity_sha256,
                    command_digests,
                    BackendFailure::before_start(HostEffectExecutorErrorId::Cancelled),
                    clock,
                ));
            }
            if effect.executable().revalidate().is_err() {
                return terminal_execution(self.finish_backend_failure(
                    effect,
                    effect_identity_sha256,
                    command_digests,
                    BackendFailure {
                        id: HostEffectExecutorErrorId::ExecutableMutation,
                        started: command_index != 0,
                        capture: model::CommandCapture::empty_failure(),
                    },
                    clock,
                ));
            }
            let capture = match self.backend.execute(
                capability,
                effect.executable(),
                command,
                &self.policy,
                cancellation,
            ) {
                Ok(capture) => capture,
                Err(failure) => {
                    return terminal_execution(self.finish_backend_failure(
                        effect,
                        effect_identity_sha256,
                        command_digests,
                        failure,
                        clock,
                    ));
                }
            };
            let digest = match capture.digest(command_index) {
                Ok(digest) => digest,
                Err(failure) => {
                    return terminal_execution(self.finish_started_failure(
                        effect,
                        effect_identity_sha256,
                        command_digests,
                        failure.id(),
                        clock,
                    ));
                }
            };
            if capture.exit_code != 0 {
                return terminal_execution(self.finish_backend_failure(
                    effect,
                    effect_identity_sha256,
                    command_digests,
                    BackendFailure {
                        id: HostEffectExecutorErrorId::ProcessFailed,
                        started: true,
                        capture,
                    },
                    clock,
                ));
            }
            command_digests.push(digest);
            if effect.executable().revalidate().is_err() {
                return terminal_execution(self.finish_started_failure(
                    effect,
                    effect_identity_sha256,
                    command_digests,
                    HostEffectExecutorErrorId::ExecutableMutation,
                    clock,
                ));
            }
        }
        Ok(ReservedExecution::Continue(command_digests))
    }
}
