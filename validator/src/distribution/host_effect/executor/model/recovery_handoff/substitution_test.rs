impl HostEffectRecoveryHandoff {
    #[cfg(test)]
    pub(super) fn substitute_permit_without_rebinding_for_test(&mut self, permit_id: String) {
        match self {
            Self::Publication {
                permit_id: current, ..
            }
            | Self::TerminalTransition {
                permit_id: current, ..
            }
            | Self::PostPublicationTerminalTransition {
                permit_id: current, ..
            }
            | Self::PostReservation {
                permit_id: current, ..
            } => *current = permit_id,
        }
    }

    #[cfg(test)]
    pub(super) fn substitute_ledger_head_without_rebinding_for_test(
        &mut self,
        replacement: HostEffectLedgerHead,
    ) {
        match self {
            Self::Publication { ledger_head, .. }
            | Self::TerminalTransition { ledger_head, .. }
            | Self::PostReservation { ledger_head, .. } => *ledger_head = replacement,
            Self::PostPublicationTerminalTransition {
                prior_ledger_head, ..
            } => *prior_ledger_head = replacement,
        }
    }

    #[cfg(test)]
    pub(super) fn substitute_originating_error_ids_without_rebinding_for_test(
        &mut self,
        replacement: Vec<HostEffectExecutorErrorId>,
    ) {
        match self {
            Self::Publication {
                originating_error_ids,
                ..
            }
            | Self::TerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostPublicationTerminalTransition {
                originating_error_ids,
                ..
            }
            | Self::PostReservation {
                originating_error_ids,
                ..
            } => *originating_error_ids = replacement,
        }
    }
}

fn valid_originating_error_chain(chain: &[HostEffectExecutorErrorId]) -> bool {
    const MAX_ORIGINATING_ERROR_CHAIN_LENGTH: usize = 4;
    !chain.is_empty()
        && chain.len() <= MAX_ORIGINATING_ERROR_CHAIN_LENGTH
        && chain
            .iter()
            .all(|id| *id != HostEffectExecutorErrorId::RecoveryRequired)
        && chain
            .iter()
            .enumerate()
            .all(|(index, id)| !chain[..index].contains(id))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CommandCaptureDigest {
    pub(super) command_index: usize,
    pub(super) exit_code: i32,
    pub(super) stdout_sha256: String,
    pub(super) stderr_sha256: String,
    pub(super) combined_sha256: String,
}

#[derive(Debug)]
pub(crate) struct CommandCapture {
    pub(super) exit_code: i32,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

impl CommandCapture {
    pub(crate) const fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub(crate) fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub(super) fn empty_failure() -> Self {
        Self {
            exit_code: -1,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    pub(super) fn digest(
        &self,
        command_index: usize,
    ) -> Result<CommandCaptureDigest, HostEffectExecutorFailure> {
        if self.stdout.len() > EXACT_OUTPUT_LIMIT_BYTES
            || self.stderr.len() > EXACT_OUTPUT_LIMIT_BYTES
        {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::OutputOverflow,
            ));
        }
        #[derive(Serialize)]
        struct Combined<'a> {
            schema: &'static str,
            command_index: usize,
            exit_code: i32,
            stdout: &'a [u8],
            stderr: &'a [u8],
        }
        Ok(CommandCaptureDigest {
            command_index,
            exit_code: self.exit_code,
            stdout_sha256: digest_bytes(&self.stdout),
            stderr_sha256: digest_bytes(&self.stderr),
            combined_sha256: digest_json(&Combined {
                schema: "harness-ultragoal.host-command-capture.v1",
                command_index,
                exit_code: self.exit_code,
                stdout: &self.stdout,
                stderr: &self.stderr,
            })?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectExecutionPolicy {
    descriptor_timeout_ms: u64,
    descriptor_stdout_limit_bytes: usize,
    descriptor_stderr_limit_bytes: usize,
    inherited_environment: bool,
    environment_entries: usize,
    environment_sha256: String,
    policy_sha256: String,
}
