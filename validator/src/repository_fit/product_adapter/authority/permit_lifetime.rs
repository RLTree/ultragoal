use super::*;

pub(crate) const DEFAULT_PERMIT_LIFETIME: u64 = 60;
pub(crate) const MAX_OUTCOME_BYTES: usize = 16 * 1024;
pub(crate) const MAX_RECOVERY_INTENT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_RECOVERY_ROWS: usize = 128;
pub(crate) const SUPPORT_LIMIT: &str = "source-built Darwin public fit apply; installed host provisioning and representative live-user proof remain separate";

/// One-use activation token for the concrete repository-fit writer. The type
/// is nameable only where the local effect adapter must consume it, while its
/// sole constructor remains private to this sealed authority module.
pub(in crate::repository_fit) struct LocalMutationGrant {
    _private: (),
}

impl LocalMutationGrant {
    pub(super) const fn issue() -> Self {
        Self { _private: () }
    }
}

#[cfg(test)]
thread_local! {
    pub(super) static AFTER_RESERVATION: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    pub(super) static AFTER_EFFECT_START_BEFORE_APPLY: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    pub(super) static AFTER_EFFECT_BEFORE_TERMINAL: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None);
    pub(super) static CONFIGURE_EFFECTS: RefCell<Option<Box<dyn FnOnce(&mut LocalEffects)>>> =
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
    fn revalidate_protected_root(&self) -> bool;
}

/// One-use high-entropy nonce carrier. It is non-cloneable, non-serializable,
/// debug-redacted, and zeroed on drop. Only its digest reaches durable state.
#[must_use = "a repository-fit production nonce must be consumed once"]
pub(crate) struct RepositoryFitApplyNonce {
    pub(crate) bytes: Vec<u8>,
}

impl RepositoryFitApplyNonce {
    pub(crate) fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, FitAdapterError> {
        let bytes = bytes.into();
        if bytes.len() < MIN_NONCE_BYTES || bytes.len() > 256 {
            return Err(adapter_error(AdapterErrorId::ApplyPermitInvalid));
        }
        Ok(Self { bytes })
    }

    pub(crate) fn sha256(&self) -> String {
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
pub(crate) struct RecoveryIntentEnvelope {
    pub(crate) schema_version: String,
    pub(crate) recovery: RecoveryTargetSpec,
}

/// Canonical recovery authority prepared before reservation. The opaque value
/// contains only bounded digests, relative canonical paths, and modes. It
/// performs no persistence and carries neither nonce nor authority key bytes.
pub(crate) struct RepositoryFitRecoveryIntent {
    pub(crate) canonical: Vec<u8>,
    pub(crate) intent_sha256: String,
    pub(crate) recovery: RecoveryTargetSpec,
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

    pub(crate) fn sha256(&self) -> &str {
        &self.intent_sha256
    }

    pub(crate) fn recovery(&self) -> &RecoveryTargetSpec {
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

pub(crate) fn canonical_recovery_intent(
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
    pub(crate) schema_version: &'static str,
    pub(crate) result_id: String,
    pub(crate) request_id: String,
    pub(crate) status: &'static str,
    pub(crate) adapter_error_id: Option<AdapterErrorId>,
    pub(crate) ledger_state: &'static str,
    pub(crate) effect_started: bool,
    pub(crate) rollback_complete: bool,
    pub(crate) apply_outcome: Option<RepositoryFitApplyOutcome>,
    pub(crate) effect: &'static str,
    pub(crate) claim_effect: &'static str,
    pub(crate) support_limit: &'static str,
}
