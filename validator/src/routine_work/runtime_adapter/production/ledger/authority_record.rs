use super::*;

pub(crate) const SCHEMA: &str = "RoutineProductionAuthorityLedger-v2";
pub(crate) const KEY_NAME: &str = "routine-authority.key";
pub(crate) const LOCK_NAME: &str = "routine-authority.lock";
pub(crate) const STATE_NAME: &str = "routine-authority.state";
pub(crate) const LOCK_MARKER: &[u8] = b"routine-production-authority-lock-v1\n";
pub(crate) const KEY_BYTES: usize = 32;
pub(crate) const MAX_STATE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RECORDS: usize = 4_096;
pub(crate) const GRANT_TTL_SECONDS: u64 = 300;
pub(crate) const RECOVERY_TTL_SECONDS: u64 = 1_800;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorityBinding {
    pub(crate) protocol_id: String,
    pub(crate) effect_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) snapshot_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AttemptState {
    Reserved,
    Started,
    Complete,
    Failed,
    Cancelled,
    Incomplete,
}

impl AttemptState {
    pub(crate) fn pending(self) -> bool {
        matches!(self, Self::Reserved | Self::Started)
    }
}

pub(crate) struct ReservationSpec {
    pub(crate) binding: AuthorityBinding,
    pub(crate) request_id: String,
    pub(crate) grant_id: String,
    pub(crate) recovery_marker: String,
    pub(crate) recovery_for: Option<String>,
    pub(crate) reuse_only: bool,
    pub(crate) reuse_preauthorization: Option<ReusePreauthorization>,
    pub(crate) output_journal: OutputProvisionJournal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputDirectoryIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) owner: u32,
    pub(crate) mode: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputComponentJournal {
    pub(crate) relative_path: String,
    pub(crate) preexisting: Option<OutputDirectoryIdentity>,
    pub(crate) creation_nonce: Option<String>,
    pub(crate) staged: Option<OutputDirectoryIdentity>,
    pub(crate) provisioned: Option<OutputDirectoryIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputProvisionJournal {
    pub(crate) root: OutputDirectoryIdentity,
    pub(crate) scopes: Vec<String>,
    pub(crate) components: Vec<OutputComponentJournal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OutputStageAmbiguity {
    pub(crate) relative_path: String,
    pub(crate) creation_nonce: String,
}

impl OutputProvisionJournal {
    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            root: OutputDirectoryIdentity {
                device: 1,
                inode: 1,
                owner: 1,
                mode: u32::from(libc::S_IFDIR) | 0o700,
            },
            scopes: Vec::new(),
            components: Vec::new(),
        }
    }
}

pub(crate) struct ReuseArtifactClaim {
    pub(crate) protocol_id: String,
    pub(crate) intent_id: String,
    pub(crate) artifact_sha256: String,
    pub(crate) result_artifact_sha256: String,
    pub(crate) mediator_witness_sha256: String,
}

/// Opaque, one-use authorization issued only after a read-only inspection of
/// one exact Complete record. It intentionally implements neither Clone nor
/// any serialization trait and is consumed by the reservation CAS.
#[must_use = "reuse preauthorization must be consumed by one exact reservation"]
pub(crate) struct ReusePreauthorization {
    pub(crate) authority_id: String,
    pub(crate) binding: AuthorityBinding,
    pub(crate) generation: u64,
    pub(crate) record_sha256: String,
    pub(crate) claims: Vec<ReuseArtifactClaim>,
}

#[derive(Clone)]
pub(crate) struct ReservationToken {
    pub(crate) binding: AuthorityBinding,
    pub(crate) request_id: String,
    pub(crate) grant_id: String,
    pub(crate) recovery_marker: String,
    pub(crate) recovery_for: Option<String>,
    pub(crate) reuse_only: bool,
    pub(crate) expires_tick: u64,
    pub(crate) output_journal: OutputProvisionJournal,
}

pub(crate) struct PendingRecovery {
    pub(crate) grant_id: String,
    pub(crate) marker: String,
    pub(crate) deadline_tick: u64,
    pub(crate) output_journal: OutputProvisionJournal,
}

pub(crate) struct FileAuthorityLedger {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::FileLedger,
}
