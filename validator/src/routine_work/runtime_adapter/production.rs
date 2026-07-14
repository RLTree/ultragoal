//! Sealed production issuance and durable routine mediation authority.

mod ledger;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use std::cell::RefCell;

use serde::Serialize;

use crate::context::LiveContext;
use crate::routine_work::digest::{digest_of, framed};
use crate::routine_work::{RoutineError, RoutineErrorId, RoutinePlan};

use super::mediator::{
    DurableAttemptAuthority, DurableSettlement, PreflightedProductionReuse, ProductionGrantBinding,
    issue_production_grant, mediate_prepared_routine_execution, preflight_production_request,
    preflight_production_reuse_input, production_grant_identity, production_recovery_identity,
};
use super::model::{PreparedRoutineExecution, RoutineEffectRequest};
use super::{RoutineCancellation, RoutineMediationResult, RoutineReuseInput};
use ledger::{
    AttemptState, AuthorityBinding, FileAuthorityLedger, ReservationSpec, ReservationToken,
    ReuseArtifactClaim,
};

/// Opaque, one-use recovery authority reconstructed only from an exact durable
/// pending record. It deliberately cannot be serialized, cloned, or forged
/// from a caller-controlled marker.
#[must_use = "recovery authority must be consumed by one exact recovery attempt"]
pub(crate) struct RoutineRecoveryAuthority {
    binding: AuthorityBinding,
    marker: String,
    deadline_tick: u64,
}

impl std::fmt::Debug for RoutineRecoveryAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RoutineRecoveryAuthority")
            .field("binding", &"[bound]")
            .field("marker", &"[redacted]")
            .field("deadline", &"[bounded]")
            .finish()
    }
}

/// The only production constructor for routine root grants.
pub(crate) struct ProductionRoutineIssuer {
    ledger: Arc<FileAuthorityLedger>,
}

impl ProductionRoutineIssuer {
    /// Opens an explicit, pre-existing, owner-only authority root. This is an
    /// effectful constructor and must never be called by help/read/query paths.
    pub(crate) fn open(authority_root: &Path) -> Result<Self, RoutineError> {
        FileAuthorityLedger::open_or_initialize(authority_root).map(|ledger| Self {
            ledger: Arc::new(ledger),
        })
    }

    fn open_existing(authority_root: &Path) -> Result<Self, RoutineError> {
        FileAuthorityLedger::open_existing(authority_root).map(|ledger| Self {
            ledger: Arc::new(ledger),
        })
    }

    /// Reconstructs bounded recovery authority only for the exact current
    /// request binding and a durable reserved-or-started record.
    pub(crate) fn pending_recovery(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: &PreparedRoutineExecution,
    ) -> Result<Option<RoutineRecoveryAuthority>, RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            return Ok(None);
        };
        preflight_production_request(context, plan, request)?;
        let binding = authority_binding(request)?;
        self.ledger.pending_recovery(&binding).map(|pending| {
            pending.map(|pending| RoutineRecoveryAuthority {
                binding,
                marker: pending.marker,
                deadline_tick: pending.deadline_tick,
            })
        })
    }

    pub(crate) fn mediate(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: PreparedRoutineExecution,
        recovery: Option<RoutineRecoveryAuthority>,
        cancellation: RoutineCancellation,
        reuse: RoutineReuseInput,
    ) -> Result<RoutineMediationResult, RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            if recovery.is_some() || !reuse.is_empty() {
                return Err(error("routine-production-noop-authority-or-reuse-present"));
            }
            return mediate_prepared_routine_execution(
                context,
                plan,
                prepared,
                None,
                cancellation,
                reuse,
            );
        };
        preflight_production_request(context, plan, &request)?;
        let require_complete_reuse_set = recovery.is_none() && !reuse.is_empty();
        let reuse = preflight_production_reuse_input(reuse, &request, require_complete_reuse_set)?;
        self.mediate_preflighted(context, plan, request, recovery, cancellation, reuse)
    }

    fn mediate_preflighted(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        request: RoutineEffectRequest,
        recovery: Option<RoutineRecoveryAuthority>,
        cancellation: RoutineCancellation,
        reuse: PreflightedProductionReuse,
    ) -> Result<RoutineMediationResult, RoutineError> {
        let (reuse, reuse_claims) = reuse.into_parts();
        let authority_binding = authority_binding(&request)?;
        let recovery_for = match recovery {
            Some(recovery)
                if recovery.binding == authority_binding
                    && now_tick()? <= recovery.deadline_tick =>
            {
                Some(recovery.marker)
            }
            Some(_) => return Err(error("routine-production-recovery-authority-stale")),
            None => None,
        };
        let reuse_only = recovery_for.is_none() && !reuse.is_empty();
        if !reuse_claims.is_empty() && !reuse_only {
            return Err(error("mediator-production-reuse-not-authenticated"));
        }
        let reuse_preauthorization = if reuse_only {
            let claims = reuse_claims
                .into_iter()
                .map(|claim| ReuseArtifactClaim {
                    protocol_id: claim.protocol_id,
                    intent_id: claim.intent_id,
                    artifact_sha256: claim.artifact_sha256,
                    result_artifact_sha256: claim.result_artifact_sha256,
                    mediator_witness_sha256: claim.mediator_witness_sha256,
                })
                .collect();
            let authorization = self.ledger.preauthorize_reuse(&authority_binding, claims)?;
            run_test_reuse_preauthorization_hook();
            Some(authorization)
        } else {
            None
        };
        let session_id = random_session_id(&authority_binding)?;
        let allowed_output_scopes = allowed_output_scopes(&request);
        let grant_binding = ProductionGrantBinding {
            session_id,
            request_id: request.request_id().to_owned(),
            protocol_id: request.protocol_id().to_owned(),
            context_id: request.context_id().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            plan_id: request.plan_id().to_owned(),
            snapshot_id: authority_binding.snapshot_id.clone(),
            allowed_output_scopes,
            recovery_for: recovery_for.clone(),
        };
        let grant_id = production_grant_identity(&grant_binding)?;
        let recovery_marker =
            production_recovery_identity(&grant_id, request.protocol_id(), request.request_id());
        let token = self.ledger.reserve(ReservationSpec {
            binding: authority_binding,
            request_id: request.request_id().to_owned(),
            grant_id,
            recovery_marker,
            recovery_for,
            reuse_only,
            reuse_preauthorization,
        })?;
        let durable = Arc::new(DurableAttempt {
            ledger: Arc::clone(&self.ledger),
            token,
        });
        let grant = issue_production_grant(grant_binding, durable)?;
        mediate_prepared_routine_execution(
            context,
            plan,
            PreparedRoutineExecution::Effect(request),
            Some(grant),
            cancellation,
            reuse,
        )
    }

    #[cfg(test)]
    pub(crate) fn test_set_reuse_preauthorization_hook(hook: impl FnOnce() + Send + 'static) {
        REUSE_PREAUTHORIZATION_TEST_HOOK.with(|slot| {
            assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
        });
    }

    #[cfg(test)]
    pub(crate) fn test_reserve_and_abandon(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: &PreparedRoutineExecution,
        started: bool,
    ) -> Result<(), RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            return Err(error("routine-production-test-effect-required"));
        };
        preflight_production_request(context, plan, request)?;
        let authority_binding = authority_binding(request)?;
        let grant_binding = ProductionGrantBinding {
            session_id: random_session_id(&authority_binding)?,
            request_id: request.request_id().to_owned(),
            protocol_id: request.protocol_id().to_owned(),
            context_id: request.context_id().to_owned(),
            candidate_id: request.candidate_id().to_owned(),
            plan_id: request.plan_id().to_owned(),
            snapshot_id: authority_binding.snapshot_id.clone(),
            allowed_output_scopes: allowed_output_scopes(request),
            recovery_for: None,
        };
        let grant_id = production_grant_identity(&grant_binding)?;
        let recovery_marker =
            production_recovery_identity(&grant_id, request.protocol_id(), request.request_id());
        let token = self.ledger.reserve(ReservationSpec {
            binding: authority_binding,
            request_id: request.request_id().to_owned(),
            grant_id,
            recovery_marker,
            recovery_for: None,
            reuse_only: false,
            reuse_preauthorization: None,
        })?;
        if started {
            self.ledger.prepare_spawn(&token)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn test_expire_pending(
        &self,
        context: &LiveContext,
        plan: &RoutinePlan,
        prepared: &PreparedRoutineExecution,
    ) -> Result<(), RoutineError> {
        let PreparedRoutineExecution::Effect(request) = prepared else {
            return Err(error("routine-production-test-effect-required"));
        };
        preflight_production_request(context, plan, request)?;
        self.ledger
            .test_expire_pending(&authority_binding(request)?)
    }

    #[cfg(test)]
    pub(crate) fn test_seed_capacity(
        &self,
        protocol_effect_count: usize,
        consumed_grant_count: usize,
    ) -> Result<(), RoutineError> {
        self.ledger
            .test_seed_capacity(protocol_effect_count, consumed_grant_count)
    }

    #[cfg(test)]
    pub(crate) const fn test_capacity_limits() -> (usize, usize) {
        FileAuthorityLedger::test_capacity_limits()
    }
}

/// No-op mediation stays outside authority initialization. Effectful requests
/// enter the sealed issuer and cannot supply a test grant.
pub(crate) fn mediate_prepared_routine_execution_production(
    authority_root: &Path,
    context: &LiveContext,
    plan: &RoutinePlan,
    prepared: PreparedRoutineExecution,
    recovery: Option<RoutineRecoveryAuthority>,
    cancellation: RoutineCancellation,
    reuse: RoutineReuseInput,
) -> Result<RoutineMediationResult, RoutineError> {
    if matches!(prepared, PreparedRoutineExecution::NoOp(_)) {
        if recovery.is_some() || !reuse.is_empty() {
            return Err(error("routine-production-noop-authority-or-reuse-present"));
        }
        return mediate_prepared_routine_execution(
            context,
            plan,
            prepared,
            None,
            cancellation,
            reuse,
        );
    }
    let PreparedRoutineExecution::Effect(request) = prepared else {
        unreachable!("no-op returned before production issuer selection")
    };
    // Malformed, non-canonical, and request-binding-invalid bytes reject before
    // the authority root is opened. Canonical supplied reuse then uses the
    // existing-only ledger path, which cannot initialize any durable file.
    preflight_production_request(context, plan, &request)?;
    let require_complete_reuse_set = recovery.is_none() && !reuse.is_empty();
    let reuse = preflight_production_reuse_input(reuse, &request, require_complete_reuse_set)?;
    let issuer = if reuse.is_empty() && recovery.is_none() {
        ProductionRoutineIssuer::open(authority_root)?
    } else {
        ProductionRoutineIssuer::open_existing(authority_root)?
    };
    issuer.mediate_preflighted(context, plan, request, recovery, cancellation, reuse)
}

#[cfg(test)]
thread_local! {
    static REUSE_PREAUTHORIZATION_TEST_HOOK: RefCell<Option<Box<dyn FnOnce() + Send>>> =
        const { RefCell::new(None) };
}

#[cfg(test)]
fn run_test_reuse_preauthorization_hook() {
    REUSE_PREAUTHORIZATION_TEST_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(test))]
fn run_test_reuse_preauthorization_hook() {}

struct DurableAttempt {
    ledger: Arc<FileAuthorityLedger>,
    token: ReservationToken,
}

impl DurableAttemptAuthority for DurableAttempt {
    fn validate_reserved(&self) -> Result<(), RoutineError> {
        self.ledger.validate_reserved(&self.token)
    }

    fn prepare_spawn(&self) -> Result<(), RoutineError> {
        self.ledger.prepare_spawn(&self.token)
    }

    fn settle(
        &self,
        outcome: DurableSettlement,
        artifacts: &BTreeMap<String, String>,
    ) -> Result<(), RoutineError> {
        let state = match outcome {
            DurableSettlement::Complete => AttemptState::Complete,
            DurableSettlement::Failed => AttemptState::Failed,
            DurableSettlement::Cancelled => AttemptState::Cancelled,
            DurableSettlement::Incomplete => AttemptState::Incomplete,
        };
        self.ledger.settle(&self.token, state, artifacts)
    }

    fn authenticates_artifact(&self, digest: &str, witness: &str) -> Result<bool, RoutineError> {
        self.ledger.authenticates(&self.token, digest, witness)
    }

    fn recovery_is_durable(&self) -> bool {
        self.token.recovery_for.is_some()
    }

    fn reuse_only(&self) -> bool {
        self.token.reuse_only
    }
}

#[derive(Serialize)]
struct EffectBinding<'a> {
    domain: &'static str,
    protocol_id: &'a str,
    context_id: &'a str,
    candidate_id: &'a str,
    plan_id: &'a str,
    result_scope: &'a str,
    intents: &'a [super::RoutineEffectIntent],
}

fn authority_binding(request: &RoutineEffectRequest) -> Result<AuthorityBinding, RoutineError> {
    let effect_id = digest_of(&EffectBinding {
        domain: "routine-production-semantic-effect-v1",
        protocol_id: request.protocol_id(),
        context_id: request.context_id(),
        candidate_id: request.candidate_id(),
        plan_id: request.plan_id(),
        result_scope: request.result_scope(),
        intents: request.intents(),
    })?;
    Ok(AuthorityBinding {
        protocol_id: request.protocol_id().to_owned(),
        effect_id,
        context_id: request.context_id().to_owned(),
        candidate_id: request.candidate_id().to_owned(),
        plan_id: request.plan_id().to_owned(),
        snapshot_id: request.snapshot_id.clone(),
    })
}

fn allowed_output_scopes(request: &RoutineEffectRequest) -> Vec<crate::routine_work::RepoPath> {
    let mut scopes = request
        .intents()
        .iter()
        .flat_map(|intent| intent.declared_output_scopes().iter().cloned())
        .collect::<Vec<_>>();
    scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    scopes.dedup();
    scopes
}

fn random_session_id(binding: &AuthorityBinding) -> Result<String, RoutineError> {
    let mut nonce = [0u8; 32];
    getrandom::fill(&mut nonce)
        .map_err(|_| error("routine-production-authority-random-unavailable"))?;
    Ok(framed(&[
        b"routine-production-session-v1",
        &nonce,
        binding.protocol_id.as_bytes(),
        binding.effect_id.as_bytes(),
    ]))
}

fn now_tick() -> Result<u64, RoutineError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| error("routine-production-trusted-time-unavailable"))
}

fn error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
