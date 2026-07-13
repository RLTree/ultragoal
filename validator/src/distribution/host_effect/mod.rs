//! Root-owned authority boundary for supported-host distribution effects.
//!
//! This module freezes the typed authority and durable-ledger interfaces. It
//! deliberately supplies no live executor, no host adapter, and no public
//! construction route. Platform adapters remain separate reviewed work, and a
//! platform that cannot execute a retained descriptor must fail as unsupported
//! before reservation, spawn, or host mutation.

mod authority;

pub(crate) use authority::{
    HostEffectAuthority, HostEffectAuthorityError, HostEffectAuthorityErrorId, HostEffectDecision,
    HostEffectPermit, HostEffectPermitBinding,
};

use super::HostCommandPlan;
use serde::Serialize;
use std::fs::File;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum HostEffectState {
    Reserved,
    InFlight,
    Settled,
    Failed,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectLedgerHead {
    generation: u64,
    head_sha256: String,
}

impl HostEffectLedgerHead {
    pub(in crate::distribution::host_effect) fn new(
        generation: u64,
        head_sha256: String,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&head_sha256) {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            generation,
            head_sha256,
        })
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn head_sha256(&self) -> &str {
        &self.head_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectReservation {
    issuer_id: String,
    ledger_id: String,
    key_id: String,
    permit_id: String,
    semantic_key_sha256: String,
    nonce_sha256: String,
    binding_sha256: String,
    expected_head_sha256: String,
    issued_at_unix_ms: u64,
    expires_at_unix_ms: u64,
}

impl HostEffectReservation {
    pub(in crate::distribution::host_effect) fn from_permit(permit: &HostEffectPermit) -> Self {
        Self {
            issuer_id: permit.issuer_id().to_owned(),
            ledger_id: permit.ledger_id().to_owned(),
            key_id: permit.key_id().to_owned(),
            permit_id: permit.permit_id().to_owned(),
            semantic_key_sha256: permit.semantic_key_sha256().to_owned(),
            nonce_sha256: permit.nonce_sha256().to_owned(),
            binding_sha256: permit.binding_sha256().to_owned(),
            expected_head_sha256: permit.binding().expected_head_sha256.clone(),
            issued_at_unix_ms: permit.binding().issued_at_unix_ms,
            expires_at_unix_ms: permit.binding().expires_at_unix_ms,
        }
    }

    pub(crate) fn permit_id(&self) -> &str {
        &self.permit_id
    }

    pub(crate) fn issuer_id(&self) -> &str {
        &self.issuer_id
    }

    pub(crate) fn ledger_id(&self) -> &str {
        &self.ledger_id
    }

    pub(crate) fn key_id(&self) -> &str {
        &self.key_id
    }

    pub(crate) fn semantic_key_sha256(&self) -> &str {
        &self.semantic_key_sha256
    }

    pub(crate) fn expected_head_sha256(&self) -> &str {
        &self.expected_head_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectLedgerRecord {
    reservation: HostEffectReservation,
    state: HostEffectState,
    record_sha256: String,
    prior_head: HostEffectLedgerHead,
    current_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl HostEffectLedgerRecord {
    pub(crate) fn reservation(&self) -> &HostEffectReservation {
        &self.reservation
    }

    pub(crate) const fn state(&self) -> HostEffectState {
        self.state
    }

    pub(crate) fn current_head(&self) -> &HostEffectLedgerHead {
        &self.current_head
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectTransition {
    permit_id: String,
    expected_state: HostEffectState,
    next_state: HostEffectState,
    expected_head: HostEffectLedgerHead,
    outcome_sha256: Option<String>,
}

impl HostEffectTransition {
    pub(in crate::distribution::host_effect) fn new(
        permit_id: String,
        expected_state: HostEffectState,
        next_state: HostEffectState,
        expected_head: HostEffectLedgerHead,
        outcome_sha256: Option<String>,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&permit_id)
            || outcome_sha256.as_ref().is_some_and(|row| !is_digest(row))
            || !allowed_transition(expected_state, next_state)
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidTransition,
            ));
        }
        Ok(Self {
            permit_id,
            expected_state,
            next_state,
            expected_head,
            outcome_sha256,
        })
    }
}

/// Cross-process implementations must reserve nonce and semantic key together,
/// publish transitions with compare-and-swap head semantics, fsync before
/// acknowledgement, and classify any uncertain started effect as Ambiguous.
pub(crate) trait DurableHostEffectLedger: Send + Sync {
    fn head(&self) -> Result<HostEffectLedgerHead, HostEffectLedgerError>;
    fn reserve(
        &self,
        reservation: HostEffectReservation,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError>;
    fn transition(
        &self,
        transition: HostEffectTransition,
    ) -> Result<HostEffectLedgerRecord, HostEffectLedgerError>;
    fn read(
        &self,
        permit_id: &str,
    ) -> Result<Option<HostEffectLedgerRecord>, HostEffectLedgerError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostEffectLedgerErrorId {
    InvalidRecord,
    InvalidTransition,
    StaleHead,
    Replay,
    Tampered,
    Io,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct HostEffectLedgerError {
    id: HostEffectLedgerErrorId,
}

impl HostEffectLedgerError {
    pub(in crate::distribution::host_effect) const fn new(id: HostEffectLedgerErrorId) -> Self {
        Self { id }
    }

    pub(crate) const fn id(&self) -> HostEffectLedgerErrorId {
        self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PinnedHostExecutableIdentity {
    canonical_path: String,
    content_sha256: String,
    device: u64,
    inode: u64,
    mode: u32,
    size: u64,
}

/// Owns the open executable object. It is intentionally neither Clone nor
/// serializable; an executor must keep this object alive through child start.
pub(crate) struct PinnedHostExecutable {
    file: File,
    identity: PinnedHostExecutableIdentity,
}

impl PinnedHostExecutable {
    pub(in crate::distribution::host_effect) fn new(
        file: File,
        identity: PinnedHostExecutableIdentity,
    ) -> Result<Self, HostEffectLedgerError> {
        if !identity.canonical_path.starts_with('/')
            || !is_digest(&identity.content_sha256)
            || identity.size == 0
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self { file, identity })
    }

    pub(in crate::distribution::host_effect) fn file(&self) -> &File {
        &self.file
    }

    pub(crate) fn identity(&self) -> &PinnedHostExecutableIdentity {
        &self.identity
    }
}

/// A ledger-reserved effect. Construction remains confined to reviewed
/// host-effect descendants and must occur only after permit verification and a
/// successful Reserved -> InFlight transition.
pub(crate) struct AuthorizedHostEffect {
    permit: HostEffectPermit,
    record: HostEffectLedgerRecord,
    executable: PinnedHostExecutable,
    plan: HostCommandPlan,
}

impl AuthorizedHostEffect {
    pub(in crate::distribution::host_effect) fn new(
        permit: HostEffectPermit,
        record: HostEffectLedgerRecord,
        executable: PinnedHostExecutable,
        plan: HostCommandPlan,
    ) -> Result<Self, HostEffectLedgerError> {
        if record.state != HostEffectState::InFlight
            || record.reservation.permit_id != permit.permit_id()
            || plan.plan_sha256() != permit.binding().command_plan_sha256
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            permit,
            record,
            executable,
            plan,
        })
    }

    pub(in crate::distribution::host_effect) fn permit(&self) -> &HostEffectPermit {
        &self.permit
    }

    pub(in crate::distribution::host_effect) fn record(&self) -> &HostEffectLedgerRecord {
        &self.record
    }

    pub(in crate::distribution::host_effect) fn executable(&self) -> &PinnedHostExecutable {
        &self.executable
    }

    pub(in crate::distribution::host_effect) fn plan(&self) -> &HostCommandPlan {
        &self.plan
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct HostEffectOutcome {
    permit_id: String,
    state: HostEffectState,
    command_output_sha256: Vec<String>,
    observed_post_state_sha256: Option<String>,
    outcome_sha256: String,
    completed_at_unix_ms: u64,
}

impl HostEffectOutcome {
    pub(in crate::distribution::host_effect) fn new(
        permit_id: String,
        state: HostEffectState,
        command_output_sha256: Vec<String>,
        observed_post_state_sha256: Option<String>,
        outcome_sha256: String,
        completed_at_unix_ms: u64,
    ) -> Result<Self, HostEffectLedgerError> {
        if !is_digest(&permit_id)
            || !matches!(
                state,
                HostEffectState::Settled | HostEffectState::Failed | HostEffectState::Ambiguous
            )
            || command_output_sha256.iter().any(|row| !is_digest(row))
            || observed_post_state_sha256
                .as_ref()
                .is_some_and(|row| !is_digest(row))
            || !is_digest(&outcome_sha256)
        {
            return Err(HostEffectLedgerError::new(
                HostEffectLedgerErrorId::InvalidRecord,
            ));
        }
        Ok(Self {
            permit_id,
            state,
            command_output_sha256,
            observed_post_state_sha256,
            outcome_sha256,
            completed_at_unix_ms,
        })
    }
}

fn allowed_transition(expected: HostEffectState, next: HostEffectState) -> bool {
    matches!(
        (expected, next),
        (HostEffectState::Reserved, HostEffectState::InFlight)
            | (HostEffectState::Reserved, HostEffectState::Failed)
            | (HostEffectState::InFlight, HostEffectState::Settled)
            | (HostEffectState::InFlight, HostEffectState::Failed)
            | (HostEffectState::InFlight, HostEffectState::Ambiguous)
            | (HostEffectState::Ambiguous, HostEffectState::Settled)
            | (HostEffectState::Ambiguous, HostEffectState::Failed)
    )
}

fn is_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    #[test]
    fn ledger_transition_graph_forbids_retry_and_terminal_revival() {
        let head = HostEffectLedgerHead::new(1, d('1')).unwrap();
        for (from, to) in [
            (HostEffectState::Reserved, HostEffectState::InFlight),
            (HostEffectState::Reserved, HostEffectState::Failed),
            (HostEffectState::InFlight, HostEffectState::Settled),
            (HostEffectState::InFlight, HostEffectState::Failed),
            (HostEffectState::InFlight, HostEffectState::Ambiguous),
            (HostEffectState::Ambiguous, HostEffectState::Settled),
            (HostEffectState::Ambiguous, HostEffectState::Failed),
        ] {
            HostEffectTransition::new(d('2'), from, to, head.clone(), None).unwrap();
        }
        for (from, to) in [
            (HostEffectState::InFlight, HostEffectState::Reserved),
            (HostEffectState::Ambiguous, HostEffectState::InFlight),
            (HostEffectState::Settled, HostEffectState::InFlight),
            (HostEffectState::Failed, HostEffectState::InFlight),
            (HostEffectState::Settled, HostEffectState::Reserved),
        ] {
            assert_eq!(
                HostEffectTransition::new(d('2'), from, to, head.clone(), None)
                    .unwrap_err()
                    .id(),
                HostEffectLedgerErrorId::InvalidTransition
            );
        }
    }
}
