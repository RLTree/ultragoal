impl HostEffectExecutionPolicy {
    /// The accepted argv binding requires a cleared environment and a one MiB
    /// output ceiling. This constructor deliberately rejects every supplied
    /// environment entry instead of inventing unbound HOME, PATH, locale, or
    /// loader variables at the executor boundary.
    pub(in crate::distribution::host_effect) fn strict(
        timeout_ms: u64,
        environment: &[(String, String)],
    ) -> Result<Self, HostEffectExecutorFailure> {
        if !environment.is_empty() {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::EnvironmentInjection,
            ));
        }
        if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidPolicy,
            ));
        }
        #[derive(Serialize)]
        struct EmptyEnvironment {
            schema: &'static str,
            inherited: bool,
            entries: usize,
        }
        let environment_sha256 = digest_json(&EmptyEnvironment {
            schema: "harness-ultragoal.cleared-host-environment.v1",
            inherited: false,
            entries: 0,
        })?;
        #[derive(Serialize)]
        struct Policy<'a> {
            schema: &'static str,
            timeout_ms: u64,
            stdout_limit_bytes: usize,
            stderr_limit_bytes: usize,
            inherited_environment: bool,
            environment_sha256: &'a str,
            shell: bool,
            process_group_containment: bool,
        }
        let policy_sha256 = digest_json(&Policy {
            schema: "harness-ultragoal.supported-host-execution-policy.v1",
            timeout_ms,
            stdout_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            stderr_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            inherited_environment: false,
            environment_sha256: &environment_sha256,
            shell: false,
            process_group_containment: true,
        })?;
        Ok(Self {
            descriptor_timeout_ms: timeout_ms,
            descriptor_stdout_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            descriptor_stderr_limit_bytes: EXACT_OUTPUT_LIMIT_BYTES,
            inherited_environment: false,
            environment_entries: 0,
            environment_sha256,
            policy_sha256,
        })
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) const fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.descriptor_timeout_ms)
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) const fn stdout_limit(&self) -> usize {
        self.descriptor_stdout_limit_bytes
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) const fn stderr_limit(&self) -> usize {
        self.descriptor_stderr_limit_bytes
    }

    pub(crate) fn environment_sha256(&self) -> &str {
        &self.environment_sha256
    }

    pub(crate) fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }
}

#[derive(Clone, Default)]
pub(crate) struct HostEffectCancellation {
    cancelled: Arc<AtomicBool>,
}

impl HostEffectCancellation {
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

pub(crate) struct HostEffectExecutionReceipt {
    effect_identity_sha256: String,
    outcome: HostEffectOutcome,
    terminal_ledger_head: HostEffectLedgerHead,
    command_output_sha256: Vec<String>,
    acknowledgement: PublicationAcknowledgementIdentity,
    acknowledgement_json: Vec<u8>,
    publication_classification: PublicationClassification,
}

impl std::fmt::Debug for HostEffectExecutionReceipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostEffectExecutionReceipt")
            .field("effect_identity_sha256", &self.effect_identity_sha256)
            .field("terminal_ledger_head", &self.terminal_ledger_head)
            .field("command_count", &self.command_output_sha256.len())
            .field(
                "publication_classification",
                &self.publication_classification.id(),
            )
            .finish_non_exhaustive()
    }
}

impl HostEffectExecutionReceipt {
    pub(super) fn new(
        effect_identity_sha256: String,
        outcome: HostEffectOutcome,
        terminal_ledger_head: HostEffectLedgerHead,
        command_output_sha256: Vec<String>,
        acknowledgement: PublicationAcknowledgementIdentity,
        acknowledgement_json: Vec<u8>,
        publication_classification: PublicationClassification,
    ) -> Self {
        Self {
            effect_identity_sha256,
            outcome,
            terminal_ledger_head,
            command_output_sha256,
            acknowledgement,
            acknowledgement_json,
            publication_classification,
        }
    }

    pub(crate) fn effect_identity_sha256(&self) -> &str {
        &self.effect_identity_sha256
    }

    pub(crate) fn outcome(&self) -> &HostEffectOutcome {
        &self.outcome
    }

    pub(crate) fn terminal_ledger_head(&self) -> &HostEffectLedgerHead {
        &self.terminal_ledger_head
    }

    pub(crate) fn command_output_sha256(&self) -> &[String] {
        &self.command_output_sha256
    }

    pub(in crate::distribution::host_effect) fn acknowledgement(
        &self,
    ) -> &PublicationAcknowledgementIdentity {
        &self.acknowledgement
    }

    pub(crate) fn acknowledgement_json(&self) -> &[u8] {
        &self.acknowledgement_json
    }

    pub(crate) fn publication_classification(&self) -> &PublicationClassification {
        &self.publication_classification
    }
}

pub(super) fn digest_json(value: &impl Serialize) -> Result<String, HostEffectExecutorFailure> {
    serde_json::to_vec(value)
        .map(|bytes| digest_bytes(&bytes))
        .map_err(|_| HostEffectExecutorFailure::new(HostEffectExecutorErrorId::Io))
}

pub(super) fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
