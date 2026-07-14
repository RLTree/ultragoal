impl RootTrustedClock for FailingClock {
    fn sample(
        &mut self,
    ) -> Result<
        TrustedTimeSample,
        crate::distribution::host_effect::lifecycle::SupportedHostLifecycleError,
    > {
        Err(lifecycle_error(
            SupportedHostLifecycleErrorId::UntrustedTime,
        ))
    }
}

enum BackendReply {
    Success(Vec<u8>, Vec<u8>),
    Failure(HostEffectExecutorErrorId, bool),
    Oversized,
}

struct ScriptedBackend {
    replies: VecDeque<BackendReply>,
    calls: usize,
}

impl ScriptedBackend {
    fn success() -> Self {
        Self {
            replies: VecDeque::from([BackendReply::Success(b"ok\n".to_vec(), Vec::new())]),
            calls: 0,
        }
    }
}

impl RetainedDescriptorProcessBackend for ScriptedBackend {
    fn execute(
        &mut self,
        _capability: &DescriptorExecutionCapability,
        _executable: &PinnedHostExecutable,
        _command: &crate::distribution::HostCommand,
        _policy: &HostEffectExecutionPolicy,
        _cancellation: &HostEffectCancellation,
    ) -> Result<CommandCapture, BackendFailure> {
        self.calls += 1;
        match self.replies.pop_front().unwrap() {
            BackendReply::Success(stdout, stderr) => Ok(CommandCapture {
                exit_code: 0,
                stdout,
                stderr,
            }),
            BackendReply::Failure(id, started) => Err(BackendFailure {
                id,
                started,
                capture: CommandCapture {
                    exit_code: -1,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                },
            }),
            BackendReply::Oversized => Ok(CommandCapture {
                exit_code: 0,
                stdout: vec![b'x'; model::EXACT_OUTPUT_LIMIT_BYTES + 1],
                stderr: Vec::new(),
            }),
        }
    }
}

fn capability() -> DescriptorExecutionCapability {
    DescriptorExecutionCapability::new(
        DescriptorExecutionPlatform::Linux,
        DescriptorExecutionPrimitive::ExecveAtEmptyPath,
        "fixture-execveat".to_owned(),
        "v1".to_owned(),
    )
    .unwrap()
}

struct FailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
}

impl DurableHostEffectLedger for FailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

struct DisplaceTargetAndFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
    target_root: PathBuf,
    displaced_root: PathBuf,
    transition_calls: Arc<AtomicU64>,
}

struct CommitThenDisplaceTargetLedger<'a> {
    inner: &'a FileHostEffectLedger,
    target_root: PathBuf,
    displaced_root: PathBuf,
    transition_calls: Arc<AtomicU64>,
}

impl DurableHostEffectLedger for CommitThenDisplaceTargetLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.transition_calls.fetch_add(1, Ordering::SeqCst);
        let terminal = self.inner.transition(transition)?;
        fs::rename(&self.target_root, &self.displaced_root)
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))?;
        Ok(terminal)
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

impl DurableHostEffectLedger for DisplaceTargetAndFailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.inner.head()
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.inner.reserve(reservation)
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.transition_calls.fetch_add(1, Ordering::SeqCst);
        fs::rename(&self.target_root, &self.displaced_root)
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

struct CommitThenFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
}
