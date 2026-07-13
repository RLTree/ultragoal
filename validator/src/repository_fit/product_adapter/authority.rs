//! Sealed production repository-fit authority candidate.
//!
//! The public dispatcher and host policy remain root-owned. This module is the
//! sole high-level source entry that may initialize the durable authority store,
//! reserve an effect, activate `LocalEffects`, and settle a terminal record.

use serde::Serialize;
use std::fmt::{Debug, Formatter};
use std::path::Path;

#[cfg(test)]
use std::cell::RefCell;

use crate::context::LiveContext;
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::{digest, PreparedFitApply};

use super::ledger::{
    FileRepositoryFitLedger, LedgerError, RepositoryFitLedgerState, ReservationDecision,
    ReservationRequest, ReservationToken,
};
use super::root_permit::{
    activate_production_permit, apply_with_root_permit, prepare_production_permit,
    production_authority_identity, production_reservation_binding, RepositoryFitApplyFailure,
    RepositoryFitApplyOutcome, MAX_PERMIT_LIFETIME, MIN_NONCE_BYTES,
};
use super::{adapter_error, AdapterErrorId, FitAdapterError};

const DEFAULT_PERMIT_LIFETIME: u64 = 60;
const MAX_OUTCOME_BYTES: usize = 16 * 1024;
const SUPPORT_LIMIT: &str =
    "source-local production authority candidate; root host policy and public dispatch absent";

/// One-use activation token for the concrete repository-fit writer. The type
/// is nameable only where the local effect adapter must consume it, while its
/// sole constructor remains private to this sealed authority module.
pub(in crate::repository_fit) struct LocalMutationGrant {
    _private: (),
}

impl LocalMutationGrant {
    const fn issue() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_RESERVATION: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    static CONFIGURE_EFFECTS: RefCell<Option<Box<dyn FnOnce(&mut LocalEffects)>>> =
        RefCell::new(None);
}

/// Root-selected trusted time. Implementations must be monotonic, independent
/// of target-controlled state, and side-effect free. The authority never falls
/// back to a process or wall clock.
pub(crate) trait RepositoryFitTrustedClock {
    fn trusted_tick(&self) -> Result<u64, FitAdapterError>;
}

/// Root-selected host-protected store policy. The returned directory must
/// already exist, be owner-only, and remain stable; merely asking for either
/// value must not create, repair, or mutate store state.
pub(crate) trait RepositoryFitAuthorityStore {
    fn protected_root(&self) -> &Path;
    fn store_id(&self) -> &str;
}

/// One-use high-entropy nonce carrier. It is non-cloneable, non-serializable,
/// debug-redacted, and zeroed on drop. Only its digest reaches durable state.
#[must_use = "a repository-fit production nonce must be consumed once"]
pub(crate) struct RepositoryFitApplyNonce {
    bytes: Vec<u8>,
}

impl RepositoryFitApplyNonce {
    pub(crate) fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, FitAdapterError> {
        let bytes = bytes.into();
        if bytes.len() < MIN_NONCE_BYTES || bytes.len() > 256 {
            return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
        }
        Ok(Self { bytes })
    }

    fn sha256(&self) -> String {
        digest(&self.bytes)
    }
}

impl Debug for RepositoryFitApplyNonce {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryFitApplyNonce")
            .field("bytes", &"[redacted]")
            .finish()
    }
}

impl Drop for RepositoryFitApplyNonce {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

/// Canonical bounded result for every production authority attempt. It never
/// contains nonce/key material, an absolute path, or raw backend diagnostics.
#[derive(Debug, Serialize)]
pub(crate) struct RepositoryFitProductionOutcome {
    schema_version: &'static str,
    result_id: String,
    request_id: String,
    status: &'static str,
    adapter_error_id: Option<AdapterErrorId>,
    ledger_state: &'static str,
    effect_started: bool,
    rollback_complete: bool,
    apply_outcome: Option<RepositoryFitApplyOutcome>,
    effect: &'static str,
    claim_effect: &'static str,
    support_limit: &'static str,
}

impl RepositoryFitProductionOutcome {
    pub(crate) fn result_id(&self) -> &str {
        &self.result_id
    }

    pub(crate) fn status(&self) -> &str {
        self.status
    }

    pub(crate) const fn error_id(&self) -> Option<AdapterErrorId> {
        self.adapter_error_id
    }

    pub(crate) const fn effect_started(&self) -> bool {
        self.effect_started
    }

    pub(crate) const fn rollback_complete(&self) -> bool {
        self.rollback_complete
    }

    pub(crate) fn to_machine_bytes(&self) -> Result<Vec<u8>, FitAdapterError> {
        let bytes = serde_json::to_vec(self)
            .map_err(|_| adapter_error(AdapterErrorId::ProjectionFailed))?;
        if bytes.len() > MAX_OUTCOME_BYTES {
            return Err(adapter_error(AdapterErrorId::ProjectionFailed));
        }
        Ok(bytes)
    }

    fn refusal(request_id: String, error: FitAdapterError) -> Self {
        Self::new(
            request_id,
            "refused",
            Some(error.id()),
            "absent",
            false,
            false,
            None,
        )
    }

    fn terminal_failure(
        request_id: String,
        error: FitAdapterError,
        state: RepositoryFitLedgerState,
        effect_started: bool,
        rollback_complete: bool,
    ) -> Self {
        let status = match state {
            RepositoryFitLedgerState::RolledBack => "rolled_back",
            RepositoryFitLedgerState::Interrupted => "interrupted",
            RepositoryFitLedgerState::Ambiguous => "ambiguous",
            RepositoryFitLedgerState::Rejected => "refused",
            RepositoryFitLedgerState::Reserved | RepositoryFitLedgerState::Committed => "ambiguous",
        };
        Self::new(
            request_id,
            status,
            Some(error.id()),
            state.name(),
            effect_started,
            rollback_complete,
            None,
        )
    }

    fn success(request_id: String, apply: RepositoryFitApplyOutcome) -> Self {
        let status = if apply.mutation_count() == 0 {
            "idempotent"
        } else {
            "applied"
        };
        Self::new(
            request_id,
            status,
            None,
            RepositoryFitLedgerState::Committed.name(),
            true,
            false,
            Some(apply),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        request_id: String,
        status: &'static str,
        adapter_error_id: Option<AdapterErrorId>,
        ledger_state: &'static str,
        effect_started: bool,
        rollback_complete: bool,
        apply_outcome: Option<RepositoryFitApplyOutcome>,
    ) -> Self {
        let effect = if effect_started {
            "workspace_write"
        } else {
            "none"
        };
        let result_id = digest(
            &serde_json::to_vec(&(
                "repository-fit-production-outcome-v1",
                &request_id,
                status,
                adapter_error_id,
                ledger_state,
                effect_started,
                rollback_complete,
                apply_outcome
                    .as_ref()
                    .map(RepositoryFitApplyOutcome::outcome_id),
                effect,
                "none",
                SUPPORT_LIMIT,
            ))
            .expect("fixed repository-fit production outcome is serializable"),
        );
        Self {
            schema_version: "RepositoryFitProductionOutcome-v1",
            result_id,
            request_id,
            status,
            adapter_error_id,
            ledger_state,
            effect_started,
            rollback_complete,
            apply_outcome,
            effect,
            claim_effect: "none",
            support_limit: SUPPORT_LIMIT,
        }
    }
}

/// The sealed authority is constructible only after request/target validation
/// and trusted-clock validation. Its private fields and opaque identity prevent
/// caller-forged permits or leases from entering the production path.
struct SealedProductionAuthority {
    ledger: FileRepositoryFitLedger,
    identity: std::sync::Arc<super::root_permit::AuthorityIdentity>,
}

impl SealedProductionAuthority {
    fn open(store: &impl RepositoryFitAuthorityStore) -> Result<Self, LedgerError> {
        let ledger =
            FileRepositoryFitLedger::open_or_initialize(store.protected_root(), store.store_id())?;
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
    clock: &impl RepositoryFitTrustedClock,
    store: &impl RepositoryFitAuthorityStore,
    nonce: RepositoryFitApplyNonce,
) -> RepositoryFitProductionOutcome {
    let request_id = prepared.request().request_id().to_owned();
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = (context, clock, store, nonce);
        return RepositoryFitProductionOutcome::refusal(
            request_id,
            adapter_error(AdapterErrorId::UnsupportedHost),
        );
    }
    #[cfg(target_vendor = "apple")]
    {
        execute_supported(context, prepared, clock, store, nonce, request_id)
    }
}

#[cfg(target_vendor = "apple")]
fn execute_supported(
    context: &LiveContext,
    prepared: PreparedFitApply,
    clock: &impl RepositoryFitTrustedClock,
    store: &impl RepositoryFitAuthorityStore,
    nonce: RepositoryFitApplyNonce,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let draft = match prepare_production_permit(context, prepared.request()) {
        Ok(draft) => draft,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let issued_tick = match clock.trusted_tick() {
        Ok(tick) => tick,
        Err(_) => {
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
            );
        }
    };
    let expires_tick = match issued_tick.checked_add(DEFAULT_PERMIT_LIFETIME) {
        Some(tick) if tick.saturating_sub(issued_tick) <= MAX_PERMIT_LIFETIME => tick,
        _ => {
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
            );
        }
    };
    let execution_tick = match clock.trusted_tick() {
        Ok(tick) if tick >= issued_tick && tick <= expires_tick => tick,
        _ => {
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
            );
        }
    };
    let authority = match SealedProductionAuthority::open(store) {
        Ok(authority) => authority,
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
        }
    };
    let nonce_sha256 = nonce.sha256();
    let binding = match production_reservation_binding(
        &draft,
        &authority.identity,
        issued_tick,
        expires_tick,
        nonce_sha256.clone(),
    ) {
        Ok(binding) => binding,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let reservation = authority.ledger.reserve(ReservationRequest {
        binding_sha256: binding.binding_sha256(),
        semantic_effect_id: binding.semantic_effect_id(),
        target_scope_id: binding.target_scope_id(),
        permit_id: binding.permit_id(),
        nonce_sha256: binding.nonce_sha256(),
        issued_tick,
        expires_tick,
    });
    let token = match reservation {
        Ok(ReservationDecision::Acquired(token)) => token,
        Ok(ReservationDecision::Existing(existing)) => {
            if existing.state() == RepositoryFitLedgerState::Reserved {
                let terminal = terminal_digest(
                    &request_id,
                    RepositoryFitLedgerState::Interrupted,
                    Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                    false,
                    false,
                    None,
                );
                return match authority.ledger.terminal(
                    existing.token(),
                    RepositoryFitLedgerState::Interrupted,
                    &terminal,
                    Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                    execution_tick,
                ) {
                    Ok(()) => RepositoryFitProductionOutcome::terminal_failure(
                        request_id,
                        adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
                        RepositoryFitLedgerState::Interrupted,
                        false,
                        false,
                    ),
                    Err(error) => RepositoryFitProductionOutcome::terminal_failure(
                        request_id,
                        error.adapter_error(),
                        RepositoryFitLedgerState::Ambiguous,
                        false,
                        false,
                    ),
                };
            }
            return RepositoryFitProductionOutcome::refusal(
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitReplayed),
            );
        }
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
        }
    };
    test_after_reservation();
    let effects = match LocalEffects::open_with_mutation_grant(
        context.worktree_root(),
        prepared.request().unix_modes().clone(),
        LocalMutationGrant::issue(),
    ) {
        Ok(mut effects) => {
            test_configure_effects(&mut effects);
            effects
        }
        Err(error) => {
            let adapter = super::kernel_error(error);
            return settle_pre_effect_failure(
                &authority.ledger,
                token,
                request_id,
                adapter,
                execution_tick,
            );
        }
    };
    let request = prepared.into_request();
    let (permit, lease) = match activate_production_permit(
        context,
        &request,
        draft,
        effects,
        authority.identity,
        issued_tick,
        expires_tick,
        nonce_sha256,
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return settle_pre_effect_failure(
                &authority.ledger,
                token,
                request_id,
                error,
                execution_tick,
            );
        }
    };
    match apply_with_root_permit(context, request, Some(permit), Some(lease), execution_tick) {
        Ok(outcome) => settle_success(
            &authority.ledger,
            token,
            request_id,
            outcome,
            execution_tick,
        ),
        Err(failure) => settle_failure(
            &authority.ledger,
            token,
            request_id,
            failure,
            execution_tick,
        ),
    }
}

#[cfg(test)]
fn test_after_reservation() {
    AFTER_RESERVATION.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
const fn test_after_reservation() {}

#[cfg(test)]
fn test_configure_effects(effects: &mut LocalEffects) {
    CONFIGURE_EFFECTS.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action(effects);
        }
    });
}

#[cfg(not(test))]
const fn test_configure_effects(_effects: &mut LocalEffects) {}

#[cfg(test)]
pub(super) fn after_reservation_for_test(action: impl FnOnce() + 'static) {
    AFTER_RESERVATION.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "an after-reservation action is already armed"
        );
    });
}

#[cfg(test)]
pub(super) fn configure_effects_for_test(action: impl FnOnce(&mut LocalEffects) + 'static) {
    CONFIGURE_EFFECTS.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "an effect configuration is already armed");
    });
}

#[cfg(target_vendor = "apple")]
fn settle_pre_effect_failure(
    ledger: &FileRepositoryFitLedger,
    token: ReservationToken,
    request_id: String,
    error: FitAdapterError,
    tick: u64,
) -> RepositoryFitProductionOutcome {
    let terminal = terminal_digest(
        &request_id,
        RepositoryFitLedgerState::Rejected,
        Some(error.id()),
        false,
        false,
        None,
    );
    match ledger.terminal(
        token,
        RepositoryFitLedgerState::Rejected,
        &terminal,
        Some(error.id()),
        tick,
    ) {
        Ok(()) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            error,
            RepositoryFitLedgerState::Rejected,
            false,
            false,
        ),
        Err(ledger_error) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            ledger_error.adapter_error(),
            RepositoryFitLedgerState::Ambiguous,
            false,
            false,
        ),
    }
}

#[cfg(target_vendor = "apple")]
fn settle_success(
    ledger: &FileRepositoryFitLedger,
    token: ReservationToken,
    request_id: String,
    outcome: RepositoryFitApplyOutcome,
    tick: u64,
) -> RepositoryFitProductionOutcome {
    let outcome_sha256 = match serde_json::to_vec(&outcome) {
        Ok(bytes) => digest(&bytes),
        Err(_) => {
            return RepositoryFitProductionOutcome::terminal_failure(
                request_id,
                adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
                RepositoryFitLedgerState::Ambiguous,
                true,
                false,
            );
        }
    };
    match ledger.terminal(
        token,
        RepositoryFitLedgerState::Committed,
        &outcome_sha256,
        None,
        tick,
    ) {
        Ok(()) => RepositoryFitProductionOutcome::success(request_id, outcome),
        Err(_) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            adapter_error(AdapterErrorId::ApplyOutcomeAmbiguous),
            RepositoryFitLedgerState::Ambiguous,
            true,
            false,
        ),
    }
}

#[cfg(target_vendor = "apple")]
fn settle_failure<E: super::root_permit::RepositoryFitPermitEffects>(
    ledger: &FileRepositoryFitLedger,
    token: ReservationToken,
    request_id: String,
    failure: RepositoryFitApplyFailure<E>,
    tick: u64,
) -> RepositoryFitProductionOutcome {
    let error = failure.error();
    let effect_started = failure.effect_started();
    let rollback_complete = failure.rollback_complete();
    let state = if !effect_started {
        RepositoryFitLedgerState::Rejected
    } else if rollback_complete {
        RepositoryFitLedgerState::RolledBack
    } else {
        RepositoryFitLedgerState::Ambiguous
    };
    let terminal = terminal_digest(
        &request_id,
        state,
        Some(error.id()),
        effect_started,
        rollback_complete,
        None,
    );
    match ledger.terminal(token, state, &terminal, Some(error.id()), tick) {
        Ok(()) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            error,
            state,
            effect_started,
            rollback_complete,
        ),
        Err(_) => RepositoryFitProductionOutcome::terminal_failure(
            request_id,
            adapter_error(if effect_started {
                AdapterErrorId::ApplyOutcomeAmbiguous
            } else {
                AdapterErrorId::ApplyOutcomeInvalid
            }),
            RepositoryFitLedgerState::Ambiguous,
            effect_started,
            false,
        ),
    }
}

fn terminal_digest(
    request_id: &str,
    state: RepositoryFitLedgerState,
    error_id: Option<AdapterErrorId>,
    effect_started: bool,
    rollback_complete: bool,
    outcome_id: Option<&str>,
) -> String {
    digest(
        &serde_json::to_vec(&(
            "repository-fit-production-terminal-v1",
            request_id,
            state,
            error_id,
            effect_started,
            rollback_complete,
            outcome_id,
        ))
        .expect("fixed terminal projection is serializable"),
    )
}
