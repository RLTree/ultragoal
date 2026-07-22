struct RecordingLedger {
    inner: Mutex<LedgerState>,
    head_calls: AtomicUsize,
    reserve_calls: AtomicUsize,
    transition_calls: AtomicUsize,
}

struct LedgerState {
    head: HostEffectLedgerHead,
    records: BTreeMap<String, HostEffectLedgerRecord>,
}

impl RecordingLedger {
    fn new(generation: u64, head_sha256: String) -> Self {
        Self {
            inner: Mutex::new(LedgerState {
                head: HostEffectLedgerHead::new(generation, head_sha256).unwrap(),
                records: BTreeMap::new(),
            }),
            head_calls: AtomicUsize::new(0),
            reserve_calls: AtomicUsize::new(0),
            transition_calls: AtomicUsize::new(0),
        }
    }

    fn observed_head(&self) -> HostEffectLedgerHead {
        self.inner.lock().unwrap().head.clone()
    }

    fn writes(&self) -> usize {
        self.reserve_calls.load(Ordering::Relaxed) + self.transition_calls.load(Ordering::Relaxed)
    }
}
