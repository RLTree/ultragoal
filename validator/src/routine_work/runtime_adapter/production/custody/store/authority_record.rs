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
    pub(crate) fn pending(self) -> bool {
        matches!(self, Self::Reserved | Self::Staged | Self::Started)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnerLease {
    pub(crate) process_id: i32,
    pub(crate) start_seconds: u64,
    pub(crate) start_microseconds: u64,
    pub(crate) nonce_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IntentBinding {
    pub(crate) intent_id: String,
    pub(crate) node_id: String,
    pub(crate) plan_order: usize,
    pub(crate) program_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChildLease {
    pub(crate) process_id: i32,
    pub(crate) process_group_id: i32,
    pub(crate) executable_sha256: String,
    pub(crate) executable_device: u64,
    pub(crate) executable_inode: u64,
    pub(crate) intent: IntentBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LaunchEntryIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
    pub(crate) owner: u32,
    pub(crate) links: u64,
    pub(crate) length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LaunchStageRecord {
    pub(crate) directory: LaunchEntryIdentity,
    pub(crate) program: LaunchEntryIdentity,
    pub(crate) marker: LaunchEntryIdentity,
    pub(crate) seal: LaunchEntryIdentity,
    pub(crate) program_sha256: String,
    pub(crate) intent: IntentBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TerminalRecord {
    pub(crate) state: AttemptState,
    pub(crate) result_sha256: String,
    pub(crate) artifacts: BTreeMap<String, String>,
    pub(crate) process_cleanup: CleanupEvidence,
    pub(crate) staged_cleanup: CleanupEvidence,
    pub(crate) prior_head_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) failure_evidence: Option<ReservationFailureEvidence>,
}

pub(in crate::routine_work::runtime_adapter::production::custody) struct ReservationSpec {
    pub(crate) binding: AuthorityBinding,
    pub(crate) request_id: String,
    pub(crate) grant_id: String,
    pub(crate) recovery_marker: String,
    pub(crate) output_journal: OutputProvisionJournal,
    pub(crate) owner: OwnerLease,
    pub(crate) intents: Vec<IntentBinding>,
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

pub(in crate::routine_work::runtime_adapter::production::custody) struct ReservationToken {
    pub(in crate::routine_work::runtime_adapter::production::custody) binding: AuthorityBinding,
    pub(in crate::routine_work::runtime_adapter::production::custody) request_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) grant_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) recovery_marker: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) expires_tick: Cell<u64>,
    pub(in crate::routine_work::runtime_adapter::production::custody) output_journal:
        OutputProvisionJournal,
    pub(in crate::routine_work::runtime_adapter::production::custody) intents: Vec<IntentBinding>,
}

pub(in crate::routine_work::runtime_adapter::production::custody) struct FileAuthorityLedger {
    #[cfg(target_vendor = "apple")]
    pub(crate) inner: supported::FileLedger,
}
