use super::*;

/// The sealed authority is constructible only after request/target validation
/// and trusted-clock validation. Its private fields and opaque identity prevent
/// caller-forged permits or leases from entering the production path.
pub(crate) struct SealedProductionAuthority {
    pub(crate) ledger: FileRepositoryFitLedger,
    pub(crate) identity: std::sync::Arc<super::super::root_permit::AuthorityIdentity>,
}

impl SealedProductionAuthority {
    pub(crate) fn open(store: &impl RepositoryFitAuthorityStore) -> Result<Self, LedgerError> {
        if !store.revalidate_protected_root() {
            return Err(LedgerError::authority_invariant());
        }
        let ledger =
            FileRepositoryFitLedger::open_or_initialize(store.protected_root(), store.store_id())?;
        if !store.revalidate_protected_root() {
            return Err(LedgerError::authority_invariant());
        }
        let identity = production_authority_identity(ledger.authority_id().to_owned())
            .map_err(|_| LedgerError::authority_invariant())?;
        Ok(Self { ledger, identity })
    }

    pub(crate) fn open_existing(
        store: &impl RepositoryFitAuthorityStore,
    ) -> Result<Self, LedgerError> {
        if !store.revalidate_protected_root() {
            return Err(LedgerError::authority_invariant());
        }
        let ledger =
            FileRepositoryFitLedger::open_existing(store.protected_root(), store.store_id())?;
        if !store.revalidate_protected_root() {
            return Err(LedgerError::authority_invariant());
        }
        let identity = production_authority_identity(ledger.authority_id().to_owned())
            .map_err(|_| LedgerError::authority_invariant())?;
        Ok(Self { ledger, identity })
    }
}

/// Consumes one exact prepared request and performs the complete production
/// source-local protocol. Unsupported hosts and every deterministic refusal
/// return before the clock/store are consulted or any mutation capability is
/// activated.
pub(crate) fn execute_prepared_apply(
    context: &LiveContext,
    prepared: PreparedFitApply,
    recovery_intent: RepositoryFitRecoveryIntent,
    clock: &impl RepositoryFitTrustedClock,
    store: &impl RepositoryFitAuthorityStore,
    nonce: RepositoryFitApplyNonce,
) -> RepositoryFitProductionOutcome {
    let request_id = prepared.request().request_id().to_owned();
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, recovery_intent, clock, store, nonce);
        return RepositoryFitProductionOutcome::refusal(
            request_id,
            adapter_error(AdapterErrorId::UnsupportedHost),
        );
    }
    #[cfg(target_vendor = "apple")]
    {
        execute_supported(
            context,
            prepared,
            recovery_intent,
            clock,
            store,
            nonce,
            request_id,
        )
    }
}

/// Explicit restart recovery. The caller must supply the exact canonical
/// pre-reservation intent and recreate the original nonce. This entry never
/// mutates the repository target; it may only append an expiry-checked terminal
/// reconciliation while holding the process-shared ledger lock.
pub(crate) fn recover_prepared_apply(
    context: &LiveContext,
    intent_bytes: &[u8],
    clock: &impl RepositoryFitTrustedClock,
    store: &impl RepositoryFitAuthorityStore,
    nonce: RepositoryFitApplyNonce,
) -> RepositoryFitProductionOutcome {
    let intent = match parse_recovery_intent(intent_bytes) {
        Ok(intent) => intent,
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(digest(intent_bytes), error);
        }
    };
    let request_id = intent.recovery().request_id.clone();
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, clock, store, nonce);
        RepositoryFitProductionOutcome::refusal(
            request_id,
            adapter_error(AdapterErrorId::UnsupportedHost),
        )
    }
    #[cfg(target_vendor = "apple")]
    {
        let recovery_tick = match clock.trusted_tick() {
            Ok(tick) => tick,
            Err(_) => {
                return RepositoryFitProductionOutcome::refusal(
                    request_id,
                    adapter_error(AdapterErrorId::ApplyPermitExpired),
                );
            }
        };
        let authority = match SealedProductionAuthority::open_existing(store) {
            Ok(authority) => authority,
            Err(error) => {
                return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
            }
        };
        let nonce_sha256 = nonce.sha256();
        let existing = match authority.ledger.lookup_by_nonce(&nonce_sha256) {
            Ok(Some(existing)) => existing,
            Ok(None) => {
                return RepositoryFitProductionOutcome::refusal(
                    request_id,
                    adapter_error(AdapterErrorId::ApplyPermitReplayed),
                );
            }
            Err(error) => {
                return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
            }
        };
        recover_existing(
            context,
            &authority.ledger,
            existing,
            &intent,
            recovery_tick,
            request_id,
        )
    }
}
