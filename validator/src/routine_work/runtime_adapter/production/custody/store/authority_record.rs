use super::*;

pub(crate) const SCHEMA: &str = "RoutineProductionCustodyStore-v4";
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
pub(in crate::routine_work::runtime_adapter::production) struct AuthorityBinding {
    pub(in crate::routine_work::runtime_adapter::production) protocol_id: String,
    pub(in crate::routine_work::runtime_adapter::production) effect_id: String,
    pub(in crate::routine_work::runtime_adapter::production) context_id: String,
    pub(in crate::routine_work::runtime_adapter::production) candidate_id: String,
    pub(in crate::routine_work::runtime_adapter::production) plan_id: String,
    pub(in crate::routine_work::runtime_adapter::production) snapshot_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum AttemptState {
    Reserved,
    Staged,
    Started,
    Complete,
    Failed,
    Cancelled,
    Incomplete,
    RolledBack,
    Ambiguous,
}

impl AttemptState {
    pub(super) fn pending(self) -> bool {
        matches!(self, Self::Reserved | Self::Staged | Self::Started)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OwnerLease {
    pub(super) process_id: i32,
    pub(super) start_seconds: u64,
    pub(super) start_microseconds: u64,
    pub(super) nonce_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IntentBinding {
    pub(super) intent_id: String,
    pub(super) node_id: String,
    pub(super) plan_order: usize,
    pub(super) program_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ChildLease {
    pub(super) process_id: i32,
    pub(super) process_group_id: i32,
    pub(super) executable_sha256: String,
    pub(super) executable_device: u64,
    pub(super) executable_inode: u64,
    pub(super) intent: IntentBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LaunchEntryIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) owner: u32,
    pub(super) links: u64,
    pub(super) length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LaunchStageRecord {
    pub(super) directory: LaunchEntryIdentity,
    pub(super) program: LaunchEntryIdentity,
    pub(super) marker: LaunchEntryIdentity,
    pub(super) seal: LaunchEntryIdentity,
    pub(super) program_sha256: String,
    pub(super) intent: IntentBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TerminalRecord {
    pub(super) state: AttemptState,
    pub(super) result_sha256: String,
    pub(super) artifacts: BTreeMap<String, String>,
    pub(super) process_cleanup: CleanupEvidence,
    pub(super) staged_cleanup: CleanupEvidence,
    pub(super) prior_head_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) failure_evidence: Option<ReservationFailureEvidence>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PublicationAmbiguity {
    pub(super) previous_head_sha256: String,
    pub(super) proposed_head_sha256: String,
    pub(super) cause: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) failure_evidence: Option<ReservationFailureEvidence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production) struct OutputDirectoryIdentity {
    pub(in crate::routine_work::runtime_adapter::production) device: u64,
    pub(in crate::routine_work::runtime_adapter::production) inode: u64,
    pub(in crate::routine_work::runtime_adapter::production) owner: u32,
    pub(in crate::routine_work::runtime_adapter::production) mode: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production) struct OutputComponentJournal {
    pub(in crate::routine_work::runtime_adapter::production) relative_path: String,
    pub(in crate::routine_work::runtime_adapter::production) preexisting:
        Option<OutputDirectoryIdentity>,
    pub(in crate::routine_work::runtime_adapter::production) creation_nonce: Option<String>,
    pub(in crate::routine_work::runtime_adapter::production) staged:
        Option<OutputDirectoryIdentity>,
    pub(in crate::routine_work::runtime_adapter::production) provisioned:
        Option<OutputDirectoryIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production) struct OutputProvisionJournal {
    pub(in crate::routine_work::runtime_adapter::production) root: OutputDirectoryIdentity,
    pub(in crate::routine_work::runtime_adapter::production) scopes: Vec<String>,
    pub(in crate::routine_work::runtime_adapter::production) components:
        Vec<OutputComponentJournal>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production) struct OutputStageAmbiguity {
    pub(in crate::routine_work::runtime_adapter::production) relative_path: String,
    pub(in crate::routine_work::runtime_adapter::production) creation_nonce: String,
}

pub(super) struct ReservationToken {
    pub(super) binding: AuthorityBinding,
    pub(super) owner: OwnerLease,
    pub(super) request_id: String,
    pub(super) grant_id: String,
    pub(super) recovery_marker: String,
    pub(super) expires_tick: Cell<u64>,
    pub(super) output_journal: OutputProvisionJournal,
    pub(super) intents: Vec<IntentBinding>,
}
