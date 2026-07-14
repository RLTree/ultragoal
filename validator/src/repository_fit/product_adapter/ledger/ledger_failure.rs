use super::*;

impl LedgerError {
    pub(crate) const fn new(id: LedgerErrorId) -> Self {
        Self { id }
    }

    pub(crate) const fn id(&self) -> LedgerErrorId {
        self.id
    }

    pub(crate) const fn authority_invariant() -> Self {
        Self::new(LedgerErrorId::Tampered)
    }

    pub(crate) const fn adapter_error(&self) -> FitAdapterError {
        let id = match self.id {
            LedgerErrorId::UnsupportedHost => AdapterErrorId::UnsupportedHost,
            LedgerErrorId::InvalidStore => AdapterErrorId::ApplyPermitInvalid,
            LedgerErrorId::Replay => AdapterErrorId::ApplyPermitReplayed,
            LedgerErrorId::ActiveLease => AdapterErrorId::ApplyLeaseInvalid,
            LedgerErrorId::Tampered | LedgerErrorId::InvalidTransition | LedgerErrorId::Io => {
                AdapterErrorId::ApplyOutcomeInvalid
            }
        };
        adapter_error(id)
    }
}

pub(crate) struct FileRepositoryFitLedger {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::FileLedger,
}

impl FileRepositoryFitLedger {
    pub(crate) fn open_or_initialize(
        root: &std::path::Path,
        store_id: &str,
    ) -> Result<Self, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            supported::FileLedger::open_or_initialize(root, store_id).map(|inner| Self { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (root, store_id);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(crate) fn open_existing(
        root: &std::path::Path,
        store_id: &str,
    ) -> Result<Self, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            supported::FileLedger::open_existing(root, store_id).map(|inner| Self { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (root, store_id);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(crate) fn authority_id(&self) -> &str {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.authority_id()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            unreachable!("unsupported hosts cannot construct a ledger")
        }
    }

    pub(crate) fn reserve(
        &self,
        request: ReservationRequest<'_>,
    ) -> Result<ReservationDecision, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.reserve(request)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = request;
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(crate) fn lookup_by_nonce(
        &self,
        nonce_sha256: &str,
    ) -> Result<Option<ExistingReservation>, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.lookup_by_nonce(nonce_sha256)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = nonce_sha256;
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(crate) fn begin_effect(
        &self,
        token: ReservationToken,
        tick: u64,
    ) -> Result<EffectOwner<'_>, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner
                .begin_effect(token, tick)
                .map(|inner| EffectOwner { inner })
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, tick);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(crate) fn reconcile_expired(
        &self,
        existing: ExistingReservation,
        recovery_intent_sha256: &str,
        recovery: &RecoveryTargetSpec,
        tick: u64,
        reconcile: impl FnOnce(RepositoryFitLedgerState, &RecoveryTargetSpec) -> RecoveryTerminal,
    ) -> Result<RepositoryFitLedgerState, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.reconcile_expired(
                existing,
                recovery_intent_sha256,
                recovery,
                tick,
                reconcile,
            )
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (existing, recovery_intent_sha256, recovery, tick, reconcile);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    pub(crate) fn terminal(
        &self,
        token: ReservationToken,
        state: RepositoryFitLedgerState,
        terminal_sha256: &str,
        error_id: Option<AdapterErrorId>,
        tick: u64,
    ) -> Result<(), LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner
                .terminal(token, state, terminal_sha256, error_id, tick)
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            let _ = (token, state, terminal_sha256, error_id, tick);
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }

    #[cfg(test)]
    pub(crate) fn snapshot_for_test(&self) -> Result<Vec<u8>, LedgerError> {
        #[cfg(target_vendor = "apple")]
        {
            self.inner.snapshot_for_test()
        }
        #[cfg(not(target_vendor = "apple"))]
        {
            Err(LedgerError::new(LedgerErrorId::UnsupportedHost))
        }
    }
}
