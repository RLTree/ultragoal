//! Sealed production repository-fit authority candidate.
//!
//! The public dispatcher and host policy remain root-owned. This module is the
//! sole high-level source entry that may initialize the durable authority store,
//! reserve an effect, activate `LocalEffects`, and settle a terminal record.

use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::path::Path;

#[cfg(test)]
use std::cell::RefCell;

use crate::context::LiveContext;
use crate::repository_fit::local::LocalEffects;
use crate::repository_fit::{
    CanonicalPath, ExpectedContent, PreparedFitApply, digest, valid_digest,
};

use super::ledger::{
    EffectOwner, FileRepositoryFitLedger, LedgerError, LedgerErrorId, RECOVERY_INTENT_SCHEMA,
    RecoveryTargetRow, RecoveryTargetSpec, RecoveryTerminal, RepositoryFitLedgerState,
    ReservationDecision, ReservationRequest, ReservationToken, canonical_recovery_intent_bytes,
};
use super::root_permit::{
    MAX_PERMIT_LIFETIME, MIN_NONCE_BYTES, ManagedAncestorContract, RepositoryFitApplyFailure,
    RepositoryFitApplyOutcome, activate_production_permit, apply_with_root_permit,
    observe_recovery_target_contract, prepare_production_permit,
    prepare_recovery_managed_ancestor_contract, prepared_managed_ancestor_contract,
    production_authority_identity, production_reservation_binding,
};
use super::{AdapterErrorId, FitAdapterError, adapter_error};

const DEFAULT_PERMIT_LIFETIME: u64 = 60;
const MAX_OUTCOME_BYTES: usize = 16 * 1024;
const MAX_RECOVERY_INTENT_BYTES: usize = 64 * 1024;
const MAX_RECOVERY_ROWS: usize = 128;
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
    static AFTER_EFFECT_START_BEFORE_APPLY: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    static AFTER_EFFECT_BEFORE_TERMINAL: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryIntentEnvelope {
    schema_version: String,
    recovery: RecoveryTargetSpec,
}

/// Canonical recovery authority prepared before reservation. The opaque value
/// contains only bounded digests, relative canonical paths, and modes. It
/// performs no persistence and carries neither nonce nor authority key bytes.
pub(crate) struct RepositoryFitRecoveryIntent {
    canonical: Vec<u8>,
    intent_sha256: String,
    recovery: RecoveryTargetSpec,
}

impl Debug for RepositoryFitRecoveryIntent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryFitRecoveryIntent")
            .field("intent", &"[bound]")
            .finish()
    }
}

impl RepositoryFitRecoveryIntent {
    pub(crate) fn to_machine_bytes(&self) -> Vec<u8> {
        self.canonical.clone()
    }

    fn sha256(&self) -> &str {
        &self.intent_sha256
    }

    fn recovery(&self) -> &RecoveryTargetSpec {
        &self.recovery
    }
}

#[cfg(target_vendor = "apple")]
pub(crate) fn prepare_recovery_intent(
    context: &LiveContext,
    prepared: &PreparedFitApply,
) -> Result<RepositoryFitRecoveryIntent, FitAdapterError> {
    let ancestors = prepare_recovery_managed_ancestor_contract(context, prepared.request())?;
    let recovery = recovery_target_spec(prepared.request(), ancestors)?;
    canonical_recovery_intent(recovery)
}

pub(crate) fn parse_recovery_intent(
    bytes: &[u8],
) -> Result<RepositoryFitRecoveryIntent, FitAdapterError> {
    if bytes.is_empty() || bytes.len() > MAX_RECOVERY_INTENT_BYTES {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let envelope: RecoveryIntentEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
    if envelope.schema_version != RECOVERY_INTENT_SCHEMA
        || !valid_recovery_target_spec(&envelope.recovery)
    {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let canonical = canonical_recovery_intent_bytes(&envelope.recovery)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    if canonical != bytes {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    Ok(RepositoryFitRecoveryIntent {
        intent_sha256: digest(&canonical),
        canonical,
        recovery: envelope.recovery,
    })
}

fn canonical_recovery_intent(
    recovery: RecoveryTargetSpec,
) -> Result<RepositoryFitRecoveryIntent, FitAdapterError> {
    if !valid_recovery_target_spec(&recovery) {
        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
    }
    let envelope = RecoveryIntentEnvelope {
        schema_version: RECOVERY_INTENT_SCHEMA.to_owned(),
        recovery,
    };
    let canonical = canonical_recovery_intent_bytes(&envelope.recovery)
        .ok_or_else(|| adapter_error(AdapterErrorId::ProjectionFailed))?;
    if canonical.is_empty() || canonical.len() > MAX_RECOVERY_INTENT_BYTES {
        return Err(adapter_error(AdapterErrorId::ProjectionFailed));
    }
    Ok(RepositoryFitRecoveryIntent {
        intent_sha256: digest(&canonical),
        recovery: envelope.recovery,
        canonical,
    })
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
            "none",
        )
    }

    fn causal_refusal(
        request_id: String,
        error: FitAdapterError,
        state: RepositoryFitLedgerState,
        effect_started: bool,
    ) -> Self {
        Self::new(
            request_id,
            "refused",
            Some(error.id()),
            state.name(),
            effect_started,
            false,
            None,
            "none",
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
            RepositoryFitLedgerState::Reserved
            | RepositoryFitLedgerState::EffectStarted
            | RepositoryFitLedgerState::Committed => "ambiguous",
        };
        Self::new(
            request_id,
            status,
            Some(error.id()),
            state.name(),
            effect_started,
            rollback_complete,
            None,
            if effect_started {
                "workspace_write"
            } else {
                "none"
            },
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
            "workspace_write",
        )
    }

    fn recovered(
        request_id: String,
        state: RepositoryFitLedgerState,
        effect_started: bool,
    ) -> Self {
        let (status, error_id) = match state {
            RepositoryFitLedgerState::Committed => ("recovered", None),
            RepositoryFitLedgerState::Interrupted => {
                ("interrupted", Some(AdapterErrorId::ApplyOutcomeAmbiguous))
            }
            RepositoryFitLedgerState::Ambiguous => {
                ("ambiguous", Some(AdapterErrorId::ApplyOutcomeAmbiguous))
            }
            _ => ("ambiguous", Some(AdapterErrorId::ApplyOutcomeInvalid)),
        };
        Self::new(
            request_id,
            status,
            error_id,
            state.name(),
            effect_started,
            false,
            None,
            "none",
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
        effect: &'static str,
    ) -> Self {
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

    fn open_existing(store: &impl RepositoryFitAuthorityStore) -> Result<Self, LedgerError> {
        let ledger =
            FileRepositoryFitLedger::open_existing(store.protected_root(), store.store_id())?;
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

#[cfg(target_vendor = "apple")]
fn execute_supported(
    context: &LiveContext,
    prepared: PreparedFitApply,
    recovery_intent: RepositoryFitRecoveryIntent,
    clock: &impl RepositoryFitTrustedClock,
    store: &impl RepositoryFitAuthorityStore,
    nonce: RepositoryFitApplyNonce,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let draft = match prepare_production_permit(context, prepared.request()) {
        Ok(draft) => draft,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let prepared_ancestors = match prepared_managed_ancestor_contract(&draft, prepared.request()) {
        Ok(contract) => contract,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    let expected_recovery = match recovery_target_spec(prepared.request(), prepared_ancestors) {
        Ok(recovery) => recovery,
        Err(error) => return RepositoryFitProductionOutcome::refusal(request_id, error),
    };
    if recovery_intent.recovery() != &expected_recovery {
        return RepositoryFitProductionOutcome::refusal(
            request_id,
            adapter_error(AdapterErrorId::ApplyPermitInvalid),
        );
    }
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
    match authority.ledger.lookup_by_nonce(&nonce_sha256) {
        Ok(Some(existing)) => {
            return refuse_existing_execution(existing, &recovery_intent, request_id);
        }
        Ok(None) => {}
        Err(error) => {
            return RepositoryFitProductionOutcome::refusal(request_id, error.adapter_error());
        }
    }
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
        recovery_intent_sha256: recovery_intent.sha256(),
        issued_tick,
        expires_tick,
        recovery: recovery_intent.recovery(),
    });
    let token = match reservation {
        Ok(ReservationDecision::Acquired(token)) => token,
        Ok(ReservationDecision::Existing(existing)) => {
            return refuse_existing_execution(existing, &recovery_intent, request_id);
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
    let effect_tick = match clock.trusted_tick() {
        Ok(tick) if tick >= execution_tick && tick <= expires_tick => tick,
        _ => {
            return settle_pre_effect_failure(
                &authority.ledger,
                token,
                request_id,
                adapter_error(AdapterErrorId::ApplyPermitExpired),
                execution_tick,
            );
        }
    };
    let owner = match authority.ledger.begin_effect(token, effect_tick) {
        Ok(owner) => owner,
        Err(error) => {
            return RepositoryFitProductionOutcome::causal_refusal(
                request_id,
                error.adapter_error(),
                RepositoryFitLedgerState::Reserved,
                false,
            );
        }
    };
    test_after_effect_start_before_apply();
    let result = apply_with_root_permit(context, request, Some(permit), Some(lease), effect_tick);
    test_after_effect_before_terminal();
    match result {
        Ok(outcome) => settle_success(owner, request_id, outcome, effect_tick),
        Err(failure) => settle_failure(owner, request_id, failure, effect_tick),
    }
}

#[cfg(target_vendor = "apple")]
fn recovery_target_spec(
    request: &super::OpaqueFitApplyRequest,
    ancestors: ManagedAncestorContract,
) -> Result<RecoveryTargetSpec, FitAdapterError> {
    let mut rows = Vec::with_capacity(request.desired.files.len());
    for desired in &request.desired.files {
        let path = desired.path.as_str();
        let post_mode = request
            .unix_modes
            .get(path)
            .copied()
            .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
        let mutation = request
            .plan
            .mutations
            .iter()
            .find(|mutation| mutation.path == desired.path);
        let (pre_sha256, pre_mode) = match mutation {
            Some(mutation) => match &mutation.expected {
                ExpectedContent::Absent => {
                    if mutation.prior.is_some()
                        || request
                            .observed_modes
                            .get(path)
                            .copied()
                            .flatten()
                            .is_some()
                    {
                        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
                    }
                    (None, None)
                }
                ExpectedContent::ExactDigest(expected) => {
                    let prior = mutation
                        .prior
                        .as_ref()
                        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
                    if digest(prior) != *expected {
                        return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
                    }
                    let mode = request
                        .observed_modes
                        .get(path)
                        .copied()
                        .flatten()
                        .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
                    (Some(expected.clone()), Some(mode))
                }
            },
            None => {
                let mode = request
                    .observed_modes
                    .get(path)
                    .copied()
                    .flatten()
                    .ok_or_else(|| adapter_error(AdapterErrorId::ApplyPermitInvalid))?;
                (Some(desired.sha256()), Some(mode))
            }
        };
        rows.push(RecoveryTargetRow {
            path: path.to_owned(),
            pre_sha256,
            pre_mode,
            post_sha256: desired.sha256(),
            post_mode,
        });
    }
    rows.sort_by(|left, right| left.path.as_bytes().cmp(right.path.as_bytes()));
    Ok(RecoveryTargetSpec {
        request_id: request.request_id().to_owned(),
        root_binding: request.root_binding().to_owned(),
        ancestors,
        rows,
    })
}

fn valid_recovery_target_spec(recovery: &RecoveryTargetSpec) -> bool {
    let leaf_paths = recovery
        .rows
        .iter()
        .map(|row| row.path.clone())
        .collect::<Vec<_>>();
    valid_digest(&recovery.request_id)
        && valid_digest(&recovery.root_binding)
        && !recovery.rows.is_empty()
        && recovery.rows.len() <= MAX_RECOVERY_ROWS
        && recovery.ancestors.valid_for_leaf_paths(&leaf_paths)
        && recovery.rows.iter().all(|row| {
            CanonicalPath::parse(&row.path).is_ok()
                && row.pre_sha256.as_deref().is_none_or(valid_digest)
                && row
                    .pre_mode
                    .is_none_or(|mode| matches!(mode, 0o644 | 0o755))
                && row.pre_sha256.is_some() == row.pre_mode.is_some()
                && valid_digest(&row.post_sha256)
                && matches!(row.post_mode, 0o644 | 0o755)
        })
        && recovery
            .rows
            .windows(2)
            .all(|rows| rows[0].path.as_bytes() < rows[1].path.as_bytes())
}

fn refuse_existing_execution(
    existing: super::ledger::ExistingReservation,
    current_intent: &RepositoryFitRecoveryIntent,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let observed_state = existing.state();
    let observed_effect_started = observed_state == RepositoryFitLedgerState::EffectStarted;
    let error = if !existing.matches_intent(current_intent.sha256(), current_intent.recovery())
        || observed_state.terminal()
    {
        AdapterErrorId::ApplyPermitReplayed
    } else {
        AdapterErrorId::ApplyLeaseInvalid
    };
    RepositoryFitProductionOutcome::causal_refusal(
        request_id,
        adapter_error(error),
        observed_state,
        observed_effect_started,
    )
}

#[cfg(target_vendor = "apple")]
fn recover_existing(
    context: &LiveContext,
    ledger: &FileRepositoryFitLedger,
    existing: super::ledger::ExistingReservation,
    intent: &RepositoryFitRecoveryIntent,
    recovery_tick: u64,
    request_id: String,
) -> RepositoryFitProductionOutcome {
    let observed_state = existing.state();
    let observed_effect_started = observed_state == RepositoryFitLedgerState::EffectStarted;
    if !existing.matches_intent(intent.sha256(), intent.recovery()) || observed_state.terminal() {
        return RepositoryFitProductionOutcome::causal_refusal(
            request_id,
            adapter_error(AdapterErrorId::ApplyPermitReplayed),
            observed_state,
            observed_effect_started,
        );
    }
    if recovery_tick <= existing.expires_tick() {
        return RepositoryFitProductionOutcome::causal_refusal(
            request_id,
            adapter_error(AdapterErrorId::ApplyLeaseInvalid),
            observed_state,
            observed_effect_started,
        );
    }
    let original_request_id = existing.request_id().to_owned();
    let prepared_root_binding = intent.recovery().root_binding.clone();
    let recovered_phase = std::cell::Cell::new(observed_state);
    match ledger.reconcile_expired(
        existing,
        intent.sha256(),
        intent.recovery(),
        recovery_tick,
        |phase, recovery| {
            recovered_phase.set(phase);
            let (state, error_id) = match phase {
                RepositoryFitLedgerState::Reserved => (
                    RepositoryFitLedgerState::Interrupted,
                    Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                ),
                RepositoryFitLedgerState::EffectStarted => {
                    match classify_recovery_target(context, &prepared_root_binding, recovery) {
                        RecoveryClassification::Preimage => (
                            RepositoryFitLedgerState::Interrupted,
                            Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                        ),
                        RecoveryClassification::Postimage => {
                            (RepositoryFitLedgerState::Committed, None)
                        }
                        RecoveryClassification::Other => (
                            RepositoryFitLedgerState::Ambiguous,
                            Some(AdapterErrorId::ApplyOutcomeAmbiguous),
                        ),
                    }
                }
                _ => (
                    RepositoryFitLedgerState::Ambiguous,
                    Some(AdapterErrorId::ApplyOutcomeInvalid),
                ),
            };
            let effect_started = phase == RepositoryFitLedgerState::EffectStarted;
            RecoveryTerminal {
                state,
                terminal_sha256: terminal_digest(
                    &recovery.request_id,
                    state,
                    error_id,
                    effect_started,
                    false,
                    None,
                ),
                error_id,
            }
        },
    ) {
        Ok(state) => RepositoryFitProductionOutcome::recovered(
            original_request_id,
            state,
            recovered_phase.get() == RepositoryFitLedgerState::EffectStarted,
        ),
        Err(error) => {
            let id = match error.id() {
                LedgerErrorId::Replay => AdapterErrorId::ApplyPermitReplayed,
                LedgerErrorId::ActiveLease => AdapterErrorId::ApplyLeaseInvalid,
                _ => error.adapter_error().id(),
            };
            RepositoryFitProductionOutcome::causal_refusal(
                request_id,
                adapter_error(id),
                observed_state,
                observed_effect_started,
            )
        }
    }
}

#[cfg(target_vendor = "apple")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryClassification {
    Preimage,
    Postimage,
    Other,
}

#[cfg(target_vendor = "apple")]
fn classify_recovery_target(
    context: &LiveContext,
    prepared_root_binding: &str,
    recovery: &RecoveryTargetSpec,
) -> RecoveryClassification {
    if context.revalidate().is_err() {
        return RecoveryClassification::Other;
    }
    let paths = match recovery
        .rows
        .iter()
        .map(|row| CanonicalPath::parse(&row.path))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(paths) => paths,
        Err(_) => return RecoveryClassification::Other,
    };
    let observation = match observe_recovery_target_contract(
        context.worktree_root(),
        &paths,
        &recovery.ancestors,
    ) {
        Ok(observation) => observation,
        Err(_) => return RecoveryClassification::Other,
    };
    if observation.root_binding != prepared_root_binding
        || observation.root_binding != recovery.root_binding
    {
        return RecoveryClassification::Other;
    }
    if context.revalidate().is_err() {
        return RecoveryClassification::Other;
    }
    let mut preimage = observation.ancestor_preimage;
    let mut postimage = observation.ancestor_postimage;
    for (observed, expected) in observation.leaves.iter().zip(&recovery.rows) {
        if observed.path != expected.path {
            return RecoveryClassification::Other;
        }
        preimage &= match (&expected.pre_sha256, expected.pre_mode) {
            (None, None) => observed.payload_sha256.is_none() && observed.mode.is_none(),
            (Some(sha256), Some(mode)) => {
                observed.valid_managed_leaf
                    && observed.payload_sha256.as_deref() == Some(sha256.as_str())
                    && observed.mode == Some(mode)
            }
            _ => false,
        };
        postimage &= observed.valid_managed_leaf
            && observed.payload_sha256.as_deref() == Some(expected.post_sha256.as_str())
            && observed.mode == Some(expected.post_mode);
    }
    if postimage {
        RecoveryClassification::Postimage
    } else if preimage {
        RecoveryClassification::Preimage
    } else {
        RecoveryClassification::Other
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
fn test_after_effect_start_before_apply() {
    AFTER_EFFECT_START_BEFORE_APPLY.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
const fn test_after_effect_start_before_apply() {}

#[cfg(test)]
fn test_after_effect_before_terminal() {
    AFTER_EFFECT_BEFORE_TERMINAL.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
const fn test_after_effect_before_terminal() {}

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
pub(super) fn after_effect_start_before_apply_for_test(action: impl FnOnce() + 'static) {
    AFTER_EFFECT_START_BEFORE_APPLY.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(
            prior.is_none(),
            "an after-effect-start action is already armed"
        );
    });
}

#[cfg(test)]
pub(super) fn after_effect_before_terminal_for_test(action: impl FnOnce() + 'static) {
    AFTER_EFFECT_BEFORE_TERMINAL.with(|slot| {
        let prior = slot.borrow_mut().replace(Box::new(action));
        assert!(prior.is_none(), "an after-effect action is already armed");
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
    owner: EffectOwner<'_>,
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
    match owner.terminal(
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
    owner: EffectOwner<'_>,
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
    match owner.terminal(state, &terminal, Some(error.id()), tick) {
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
