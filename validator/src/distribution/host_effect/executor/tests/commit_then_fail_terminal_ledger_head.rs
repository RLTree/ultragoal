impl DurableHostEffectLedger for CommitThenFailTerminalLedger<'_> {
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
        self.inner.transition(transition)?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

struct MutateCommitThenFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
}

impl DurableHostEffectLedger for MutateCommitThenFailTerminalLedger<'_> {
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
        mut transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        transition.next_state = HostEffectState::Failed;
        self.inner.transition(transition)?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        self.inner.read(permit_id)
    }
}

#[derive(Clone, Copy)]
enum RecoveryObservationMode {
    Current,
    Error,
    SubstituteUnrelatedRecord,
    StaleHead,
}

struct AdvanceUnrelatedPermitThenFailTerminalLedger<'a> {
    inner: &'a FileHostEffectLedger,
    authority: HostEffectAuthority,
    unrelated_binding: HostEffectPermitBinding,
    stale_head: HostEffectLedgerHead,
    commit_terminal: bool,
    repeat_advance: bool,
    observation_mode: RecoveryObservationMode,
    advanced: AtomicBool,
    unrelated_record: Mutex<Option<HostEffectLedgerRecord>>,
}

impl AdvanceUnrelatedPermitThenFailTerminalLedger<'_> {
    fn advance_unrelated_permit(&self) -> Result<(), HostEffectLedgerError> {
        let mut binding = self.unrelated_binding.clone();
        binding.expected_head_sha256 = self.inner.head()?.head_sha256().to_owned();
        let (_permit, reservation) = self
            .authority
            .issue(binding)
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))?;
        let reserved = self.inner.reserve(reservation)?;
        let latest = if self.repeat_advance {
            self.inner.transition(
                HostEffectTransition::new(
                    reserved.reservation().permit_id().to_owned(),
                    HostEffectState::Reserved,
                    HostEffectState::InFlight,
                    reserved.current_head().clone(),
                    None,
                )
                .unwrap(),
            )?
        } else {
            reserved
        };
        *self
            .unrelated_record
            .lock()
            .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))? = Some(latest);
        self.advanced.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl DurableHostEffectLedger for AdvanceUnrelatedPermitThenFailTerminalLedger<'_> {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        if self.advanced.load(Ordering::SeqCst) {
            match self.observation_mode {
                RecoveryObservationMode::Error => {
                    return Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io));
                }
                RecoveryObservationMode::StaleHead => return Ok(self.stale_head.clone()),
                RecoveryObservationMode::Current
                | RecoveryObservationMode::SubstituteUnrelatedRecord => {}
            }
        }
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
        if self.commit_terminal {
            self.inner.transition(transition)?;
        }
        self.advance_unrelated_permit()?;
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        if self.advanced.load(Ordering::SeqCst) {
            match self.observation_mode {
                RecoveryObservationMode::Error => {
                    return Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io));
                }
                RecoveryObservationMode::SubstituteUnrelatedRecord => {
                    return self
                        .unrelated_record
                        .lock()
                        .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
                        .map(|record| record.clone());
                }
                RecoveryObservationMode::Current | RecoveryObservationMode::StaleHead => {}
            }
        }
        self.inner.read(permit_id)
    }
}

struct UnavailableObservationLedger;
