impl DurableHostEffectLedger for RecordingLedger {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError> {
        self.head_calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.inner.lock().unwrap().head.clone())
    }

    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.reserve_calls.fetch_add(1, Ordering::Relaxed);
        let mut inner = self.inner.lock().unwrap();
        if reservation.expected_head_sha256() != inner.head.head_sha256() {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::StaleHead,
            ));
        }
        let prior = inner.head.clone();
        let next = HostEffectLedgerHead::new(
            prior.generation() + 1,
            digest(format!("reserve:{}", reservation.permit_id()).as_bytes()),
        )?;
        let record = HostEffectLedgerRecord {
            reservation: reservation.clone(),
            state: HostEffectState::Reserved,
            record_sha256: digest(format!("record:{}", reservation.permit_id()).as_bytes()),
            prior_head: prior,
            current_head: next.clone(),
            outcome_sha256: None,
        };
        inner.head = next;
        inner
            .records
            .insert(reservation.permit_id().to_owned(), record.clone());
        Ok(record)
    }

    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError> {
        self.transition_calls.fetch_add(1, Ordering::Relaxed);
        let mut inner = self.inner.lock().unwrap();
        if transition.expected_head != inner.head {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::StaleHead,
            ));
        }
        let current = inner
            .records
            .get(&transition.permit_id)
            .cloned()
            .ok_or_else(|| HostEffectLedgerError::new(HostEffectLedgerErrorId::InvalidRecord))?;
        if current.state() != transition.expected_state
            || transition.next_state != HostEffectState::InFlight
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidTransition,
            ));
        }
        let next = HostEffectLedgerHead::new(
            inner.head.generation() + 1,
            digest(format!("transition:{}", transition.permit_id).as_bytes()),
        )?;
        let record = HostEffectLedgerRecord {
            reservation: current.reservation().clone(),
            state: transition.next_state,
            record_sha256: digest(format!("in-flight:{}", transition.permit_id).as_bytes()),
            prior_head: inner.head.clone(),
            current_head: next.clone(),
            outcome_sha256: transition.outcome_sha256,
        };
        inner.head = next;
        inner.records.insert(transition.permit_id, record.clone());
        Ok(record)
    }

    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError> {
        Ok(self.inner.lock().unwrap().records.get(permit_id).cloned())
    }
}

struct ProbeClock {
    samples: VecDeque<TrustedTimeSample>,
    calls: usize,
}

impl ProbeClock {
    fn good(start: u64) -> Self {
        Self {
            samples: VecDeque::from([
                TrustedTimeSample::new("root-monotonic-clock".to_owned(), 1, 1, start).unwrap(),
                TrustedTimeSample::new("root-monotonic-clock".to_owned(), 1, 2, start + 1).unwrap(),
            ]),
            calls: 0,
        }
    }
}

impl RootTrustedClock for ProbeClock {
    fn sample(&mut self) -> Result<TrustedTimeSample, SupportedHostLifecycleError> {
        self.calls += 1;
        self.samples
            .pop_front()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::UntrustedTime))
    }
}

struct ProbeTarget {
    observations: VecDeque<ObservedTargetIdentity>,
    acquisitions: Arc<AtomicUsize>,
    revalidations: Arc<AtomicUsize>,
}

impl ProbeTarget {
    fn stable(target: &ObservedTargetIdentity) -> Self {
        Self {
            observations: VecDeque::from([target.clone(), target.clone(), target.clone()]),
            acquisitions: Arc::new(AtomicUsize::new(0)),
            revalidations: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl HostTargetObserver for ProbeTarget {
    fn acquire(
        &mut self,
        _expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError> {
        self.acquisitions.fetch_add(1, Ordering::Relaxed);
        let identity = self
            .observations
            .pop_front()
            .ok_or_else(|| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        Ok(Box::new(ProbeTargetLease {
            identity,
            observations: std::mem::take(&mut self.observations),
            revalidations: Arc::clone(&self.revalidations),
        }))
    }
}

struct ProbeTargetLease {
    identity: ObservedTargetIdentity,
    observations: VecDeque<ObservedTargetIdentity>,
    revalidations: Arc<AtomicUsize>,
}
