use super::*;

pub(crate) const PERMIT_DOMAIN: &str = "repository-fit-root-apply-permit-v1";
#[cfg(test)]
pub(crate) const AUTHORITY_DOMAIN: &str = "repository-fit-root-apply-authority-v1";
pub(crate) const ROLLBACK_POLICY: &str = "complete-exact-prestate-or-ambiguous-v1";
pub(crate) const MAX_PERMIT_LIFETIME: u64 = 300;
pub(crate) const MIN_NONCE_BYTES: usize = 32;
pub(crate) const MAX_FENCE_ENTRIES: usize = 100_000;
pub(crate) const MAX_FENCE_DEPTH: usize = 64;
pub(crate) const MAX_FENCE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_FENCE_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const CREATED_MANAGED_ANCESTOR_MODE: u32 = 0o755;
pub(crate) const MAX_TARGET_ENTRY_SCAN: usize = 4 * 1024;
pub(crate) const MAX_TARGET_NAME_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_MANAGED_ANCESTOR_CONTRACT_ROWS: usize = 512;

#[cfg(test)]
thread_local! {
    pub(super) static BEFORE_POSTFLIGHT_OBSERVATION: RefCell<Option<Box<dyn FnOnce()>>> =
        RefCell::new(None);
    pub(super) static BEFORE_FINAL_GREEN_OBSERVATION: RefCell<Option<Box<dyn FnOnce()>>> =
        RefCell::new(None);
    pub(super) static TARGET_CAPTURE_HOOK: RefCell<Option<TargetCaptureHook>> = RefCell::new(None);
    pub(super) static PROTECTED_CAPTURE_HOOK: RefCell<Option<ProtectedCaptureHook>> = RefCell::new(None);
    pub(super) static RECONCILIATION_TARGET_HOOK: RefCell<Option<ReconciliationTargetHook>> =
        RefCell::new(None);
}

#[cfg(test)]
pub(crate) struct TargetCaptureHook {
    pub(crate) phase: TargetCapturePhase,
    pub(crate) path: String,
    pub(crate) action: Box<dyn FnOnce()>,
}

#[cfg(test)]
pub(crate) struct ProtectedCaptureHook {
    pub(crate) boundary: ProtectedCaptureBoundary,
    pub(crate) phase: ProtectedCapturePhase,
    pub(crate) path: Vec<u8>,
    pub(crate) action: Box<dyn FnOnce()>,
}

#[cfg(test)]
pub(crate) struct ReconciliationTargetHook {
    pub(crate) phase: ReconciliationTargetPhase,
    pub(crate) action: Box<dyn FnOnce()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetCapturePhase {
    AfterNamedBeforeOpen,
    AfterParentHeldBeforeDescend,
    AfterLeafHeldBeforeRead,
    AfterLeafRevalidated,
    AfterMissingBeforeRecheck,
    BeforeFinalChainRecheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProtectedCaptureBoundary {
    PermitIssuance,
    PermitIssuanceRecheck,
    ImmediatePreEffect,
    ImmediatePreEffectRecheck,
    Postflight,
    PostflightRecheck,
    FinalGreen,
    FinalGreenRecheck,
    Rollback,
    AmbiguityReconciliation,
    AmbiguityReconciliationRecheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProtectedCapturePhase {
    AfterEnumerationBeforeChildOpen,
    AfterDirectoryHeldBeforeDescend,
    AfterRegularRowRevalidated,
    BeforeFinalRecheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReconciliationTargetPhase {
    AuthorizedRevalidation,
    FirstTarget,
    ProtectedAfter,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PermitBinding {
    pub(crate) request_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) repository_root_id: String,
    pub(crate) worktree_root_id: String,
    pub(crate) root_binding: String,
    pub(crate) plan_record_sha256: String,
    pub(crate) plan_sha256: String,
    pub(crate) accepted_plan_sha256: String,
    pub(crate) desired_state_sha256: String,
    pub(crate) source_manifest_sha256: String,
    pub(crate) source_catalog_sha256: String,
    pub(crate) source_authority_sha256: String,
    pub(crate) target_prestate_sha256: String,
    pub(crate) allowed_mutation_set_sha256: String,
    pub(crate) rollback_policy_sha256: String,
    pub(crate) protected_prestate_sha256: String,
}

pub(crate) struct AuthorityIdentity {
    pub(crate) authority_id: String,
}

impl Debug for AuthorityIdentity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthorityIdentity")
            .field("authority_id", &"[bound]")
            .finish()
    }
}

impl AuthorityIdentity {
    pub(crate) fn id(&self) -> &str {
        &self.authority_id
    }
}

/// Descriptor-bound pre-effect material prepared before the durable store is
/// initialized. Keeping the descriptor chains alive closes target replacement
/// between validation, reservation, and production lease activation.
pub(crate) struct PreparedProductionPermit {
    pub(crate) binding: PermitBinding,
    pub(crate) target_prestate: TargetSnapshot,
    pub(crate) target_chain: TargetDescriptorChain,
    pub(crate) protected_prestate: ProtectedSnapshot,
}

/// Bounded durable identifiers for one exact permit reservation. No raw nonce,
/// path, target bytes, or authority key material crosses this projection.
pub(crate) struct ProductionReservationBinding {
    pub(crate) binding_sha256: String,
    pub(crate) semantic_effect_id: String,
    pub(crate) target_scope_id: String,
    pub(crate) permit_id: String,
    pub(crate) nonce_sha256: String,
}

impl ProductionReservationBinding {
    pub(crate) fn binding_sha256(&self) -> &str {
        &self.binding_sha256
    }

    pub(crate) fn semantic_effect_id(&self) -> &str {
        &self.semantic_effect_id
    }

    pub(crate) fn target_scope_id(&self) -> &str {
        &self.target_scope_id
    }

    pub(crate) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(crate) fn nonce_sha256(&self) -> &str {
        &self.nonce_sha256
    }
}

/// Opaque root authority for one exact request instance. It intentionally
/// implements neither Clone, Copy, Serialize, nor Deserialize.
#[must_use = "a repository-fit apply permit must be consumed or explicitly discarded"]
pub(crate) struct RepositoryFitApplyPermit {
    pub(crate) permit_id: String,
    pub(crate) binding: PermitBinding,
    pub(crate) issued_tick: u64,
    pub(crate) expires_tick: u64,
    pub(crate) nonce_sha256: String,
    pub(crate) authority: Arc<AuthorityIdentity>,
    pub(crate) seal: Arc<ApplyRequestSeal>,
    pub(crate) target_prestate: TargetSnapshot,
    pub(crate) target_chain: TargetDescriptorChain,
    pub(crate) protected_prestate: ProtectedSnapshot,
}

impl Debug for RepositoryFitApplyPermit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RepositoryFitApplyPermit")
            .field("permit_id", &"[bound]")
            .field("binding", &"[bound]")
            .field("issued_tick", &self.issued_tick)
            .field("expires_tick", &self.expires_tick)
            .field("nonce", &"[redacted]")
            .field("authority", &"[redacted]")
            .finish()
    }
}
