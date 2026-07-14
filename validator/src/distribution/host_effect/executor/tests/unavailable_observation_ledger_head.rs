impl DurableHostEffectLedger for UnavailableObservationLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn reserve(
        &self,
        _reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn transition(
        &self,
        _transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }

    fn read(
        &self,
        _permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
    }
}

#[derive(Clone, Copy)]
enum PostReservationObservationMode {
    Stable,
    ReadError,
}

struct PostReservationObservationLedger {
    expected_permit_id: String,
    observed_head: HostEffectLedgerHead,
    observed_record: HostEffectLedgerRecord,
    mode: PostReservationObservationMode,
    head_calls: AtomicU64,
    read_calls: AtomicU64,
}

impl PostReservationObservationLedger {
    fn stable(
        expected_permit_id: String,
        observed_head: HostEffectLedgerHead,
        observed_record: HostEffectLedgerRecord,
    ) -> Self {
        Self {
            expected_permit_id,
            observed_head,
            observed_record,
            mode: PostReservationObservationMode::Stable,
            head_calls: AtomicU64::new(0),
            read_calls: AtomicU64::new(0),
        }
    }

    fn read_error(
        expected_permit_id: String,
        observed_head: HostEffectLedgerHead,
        observed_record: HostEffectLedgerRecord,
    ) -> Self {
        Self {
            expected_permit_id,
            observed_head,
            observed_record,
            mode: PostReservationObservationMode::ReadError,
            head_calls: AtomicU64::new(0),
            read_calls: AtomicU64::new(0),
        }
    }
}

impl DurableHostEffectLedger for PostReservationObservationLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.head_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed_head.clone())
    }

    fn reserve(
        &self,
        _reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
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
        self.read_calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(permit_id, self.expected_permit_id);
        match self.mode {
            PostReservationObservationMode::Stable => Ok(Some(self.observed_record.clone())),
            PostReservationObservationMode::ReadError => {
                Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
            }
        }
    }
}

struct ReplayThenWrongPermitObservationLedger {
    expected_permit_id: String,
    replay_record: HostEffectLedgerRecord,
    observed_head: HostEffectLedgerHead,
    wrong_permit_record: HostEffectLedgerRecord,
    head_calls: AtomicU64,
    read_calls: AtomicU64,
}

impl DurableHostEffectLedger for ReplayThenWrongPermitObservationLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.head_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.observed_head.clone())
    }

    fn reserve(
        &self,
        _reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        Err(HostEffectLedgerError::new(HostEffectLedgerErrorId::Io))
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
        assert_eq!(permit_id, self.expected_permit_id);
        let call = self.read_calls.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            Ok(Some(self.replay_record.clone()))
        } else {
            Ok(Some(self.wrong_permit_record.clone()))
        }
    }
}
