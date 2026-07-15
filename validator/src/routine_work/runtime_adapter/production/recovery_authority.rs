use super::*;

/// Opaque, one-use recovery authority reconstructed only from an exact durable
/// pending record. It deliberately cannot be serialized, cloned, or forged
/// from a caller-controlled marker.
#[must_use = "recovery authority must be consumed by one exact recovery attempt"]
pub(super) struct RoutineRecoveryAuthority {
    pub(super) binding: AuthorityBinding,
    pub(super) grant_id: String,
    pub(super) marker: String,
    pub(super) deadline_tick: u64,
}

impl std::fmt::Debug for RoutineRecoveryAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RoutineRecoveryAuthority")
            .field("binding", &"[bound]")
            .field("grant", &"[bound]")
            .field("marker", &"[redacted]")
            .field("deadline", &"[bounded]")
            .finish()
    }
}

/// The only production constructor for routine root grants.
pub(super) struct ProductionRoutineIssuer {
    pub(super) ledger: Arc<FileAuthorityLedger>,
    pub(super) launch_root: PathBuf,
}
