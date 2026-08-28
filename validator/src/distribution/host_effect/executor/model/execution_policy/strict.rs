impl HostEffectExecutionPolicy {
    /// The established private fixture policy permits only a cleared environment.
    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn strict(
        timeout_ms: u64,
        environment: &[(String, String)],
    ) -> Result<Self, HostEffectExecutorFailure> {
        if !environment.is_empty() {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::EnvironmentInjection,
            ));
        }
        Self::from_bound_environment(timeout_ms, environment)
    }

    pub(in crate::distribution::host_effect) fn strict_isolated_codex_home(
        timeout_ms: u64,
        isolated_home: &std::path::Path,
    ) -> Result<Self, HostEffectExecutorFailure> {
        let home = isolated_home.canonicalize().map_err(|_| {
            HostEffectExecutorFailure::new(HostEffectExecutorErrorId::EnvironmentInjection)
        })?;
        if !home.is_absolute() {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::EnvironmentInjection,
            ));
        }
        let value = home.display().to_string();
        let environment = [
            ("CODEX_HOME".to_owned(), value.clone()),
            ("HOME".to_owned(), value),
        ];
        Self::from_bound_environment(timeout_ms, &environment)
    }

    pub(in crate::distribution::host_effect) fn strict_personal_codex_home(
        timeout_ms: u64,
        personal_home: &std::path::Path,
    ) -> Result<Self, HostEffectExecutorFailure> {
        let home = personal_home.canonicalize().map_err(|_| {
            HostEffectExecutorFailure::new(HostEffectExecutorErrorId::EnvironmentInjection)
        })?;
        let codex_home = home.join(".codex").canonicalize().map_err(|_| {
            HostEffectExecutorFailure::new(HostEffectExecutorErrorId::EnvironmentInjection)
        })?;
        if home != personal_home || codex_home != home.join(".codex") {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::EnvironmentInjection,
            ));
        }
        let environment = [
            ("CODEX_HOME".to_owned(), codex_home.display().to_string()),
            ("HOME".to_owned(), home.display().to_string()),
        ];
        Self::from_bound_environment(timeout_ms, &environment)
    }

    pub(in crate::distribution::host_effect) fn strict_runtime(
        timeout_ms: u64,
    ) -> Result<Self, HostEffectExecutorFailure> {
        Self::from_bound_environment(timeout_ms, &[])
    }

    fn from_bound_environment(
        timeout_ms: u64,
        environment: &[(String, String)],
    ) -> Result<Self, HostEffectExecutorFailure> {
        if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
            return Err(HostEffectExecutorFailure::new(
                HostEffectExecutorErrorId::InvalidPolicy,
            ));
        }
        #[derive(Serialize)]
        struct BoundEnvironment<'a> {
            schema: &'static str,
            inherited: bool,
            entries: usize,
            values: &'a [(String, String)],
        }
        let environment_sha256 = digest_json(&BoundEnvironment {
            schema: "harness-ultragoal.cleared-host-environment.v1",
            inherited: false,
            entries: environment.len(),
            values: environment,
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
            environment_entries: environment.len(),
            environment_sha256,
            policy_sha256,
        })
    }

    pub(in crate::distribution::host_effect) const fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.descriptor_timeout_ms)
    }

    pub(in crate::distribution::host_effect) const fn stdout_limit(&self) -> usize {
        self.descriptor_stdout_limit_bytes
    }

    pub(in crate::distribution::host_effect) const fn stderr_limit(&self) -> usize {
        self.descriptor_stderr_limit_bytes
    }

    pub(crate) fn environment_sha256(&self) -> &str {
        &self.environment_sha256
    }

    pub(crate) fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }
}

#[cfg(unix)]
static PROCESS_SIGNAL_TARGET: std::sync::atomic::AtomicPtr<HostEffectCancellation> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());
#[cfg(unix)]
static PROCESS_SIGNAL_INSTALLATION: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    #[cfg(unix)]
    pub(crate) fn install_process_signal_source(
        &self,
    ) -> Result<HostEffectSignalGuard, &'static str> {
        let installation = PROCESS_SIGNAL_INSTALLATION
            .lock()
            .map_err(|_| "process cancellation signal custody is unavailable")?;
        let cancellation = Box::new(self.clone());
        PROCESS_SIGNAL_TARGET.store(
            std::ptr::from_ref(cancellation.as_ref()).cast_mut(),
            Ordering::SeqCst,
        );
        let mut action = unsafe { std::mem::zeroed::<libc::sigaction>() };
        action.sa_sigaction = record_process_cancellation as *const () as usize;
        action.sa_flags = 0;
        unsafe { libc::sigemptyset(&mut action.sa_mask) };
        let mut previous_interrupt = unsafe { std::mem::zeroed::<libc::sigaction>() };
        let mut previous_terminate = unsafe { std::mem::zeroed::<libc::sigaction>() };
        if unsafe { libc::sigaction(libc::SIGINT, &action, &mut previous_interrupt) } != 0 {
            PROCESS_SIGNAL_TARGET.store(std::ptr::null_mut(), Ordering::SeqCst);
            return Err("SIGINT cancellation source is unavailable");
        }
        if unsafe { libc::sigaction(libc::SIGTERM, &action, &mut previous_terminate) } != 0 {
            unsafe { libc::sigaction(libc::SIGINT, &previous_interrupt, std::ptr::null_mut()) };
            PROCESS_SIGNAL_TARGET.store(std::ptr::null_mut(), Ordering::SeqCst);
            return Err("SIGTERM cancellation source is unavailable");
        }
        Ok(HostEffectSignalGuard {
            previous_interrupt,
            previous_terminate,
            _cancellation: cancellation,
            _installation: installation,
        })
    }
}

#[cfg(unix)]
extern "C" fn record_process_cancellation(_signal: libc::c_int) {
    let target = PROCESS_SIGNAL_TARGET.load(Ordering::SeqCst);
    if !target.is_null() {
        // SAFETY: the installation guard retains the boxed token until both
        // handlers have been restored and the target pointer has been cleared.
        unsafe { (*target).cancel() };
    }
}

#[cfg(unix)]
pub(crate) struct HostEffectSignalGuard {
    previous_interrupt: libc::sigaction,
    previous_terminate: libc::sigaction,
    _cancellation: Box<HostEffectCancellation>,
    _installation: std::sync::MutexGuard<'static, ()>,
}

#[cfg(unix)]
impl Drop for HostEffectSignalGuard {
    fn drop(&mut self) {
        unsafe {
            libc::sigaction(
                libc::SIGTERM,
                &self.previous_terminate,
                std::ptr::null_mut(),
            );
            libc::sigaction(libc::SIGINT, &self.previous_interrupt, std::ptr::null_mut());
        }
        PROCESS_SIGNAL_TARGET.store(std::ptr::null_mut(), Ordering::SeqCst);
    }
}

pub(crate) struct HostEffectExecutionReceipt {
    effect_identity_sha256: String,
    _outcome: HostEffectOutcome,
    terminal_record: HostEffectLedgerRecord,
    terminal_ledger_head: HostEffectLedgerHead,
    command_output_sha256: Vec<String>,
    _acknowledgement: PublicationAcknowledgementIdentity,
    _acknowledgement_json: Vec<u8>,
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
        _terminal_record: HostEffectLedgerRecord,
        terminal_ledger_head: HostEffectLedgerHead,
        command_output_sha256: Vec<String>,
        acknowledgement: PublicationAcknowledgementIdentity,
        acknowledgement_json: Vec<u8>,
        publication_classification: PublicationClassification,
    ) -> Self {
        Self {
            effect_identity_sha256,
            _outcome: outcome,
            terminal_record: _terminal_record,
            terminal_ledger_head,
            command_output_sha256,
            _acknowledgement: acknowledgement,
            _acknowledgement_json: acknowledgement_json,
            publication_classification,
        }
    }

    #[cfg(test)]
    pub(crate) fn effect_identity_sha256(&self) -> &str {
        &self.effect_identity_sha256
    }

    #[cfg(test)]
    pub(crate) fn outcome(&self) -> &HostEffectOutcome {
        &self._outcome
    }

    #[cfg(test)]
    pub(crate) fn terminal_ledger_head(&self) -> &HostEffectLedgerHead {
        &self.terminal_ledger_head
    }

    pub(crate) fn command_output_sha256(&self) -> &[String] {
        &self.command_output_sha256
    }

    pub(crate) fn recovery_handoff(&self) -> HostEffectRecoveryHandoff {
        HostEffectRecoveryHandoff::terminal_transition(TerminalTransitionRecoveryRequest {
            effect_identity_sha256: self.effect_identity_sha256.clone(),
            permit_id: self._outcome.permit_id.clone(),
            ledger_head: self.terminal_ledger_head.clone(),
            ledger_record: self.terminal_record.clone(),
            exact_current_ledger_observation: true,
            outcome: self._outcome.clone(),
            originating_error_ids: vec![HostEffectExecutorErrorId::RecoveryRequired],
            classification: HostEffectTerminalRecoveryClassification::TerminalCommittedAndVerified,
        })
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn acknowledgement(
        &self,
    ) -> &PublicationAcknowledgementIdentity {
        &self._acknowledgement
    }

    #[cfg(test)]
    pub(crate) fn acknowledgement_json(&self) -> &[u8] {
        &self._acknowledgement_json
    }

    #[cfg(test)]
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
