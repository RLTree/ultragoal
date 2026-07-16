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
pub(in crate::routine_work::runtime_adapter::production::custody) enum AttemptState {
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
    pub(in crate::routine_work::runtime_adapter::production::custody) fn pending(self) -> bool {
        matches!(self, Self::Reserved | Self::Staged | Self::Started)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct OwnerLease {
    pub(in crate::routine_work::runtime_adapter::production::custody) process_id: i32,
    pub(in crate::routine_work::runtime_adapter::production::custody) start_seconds: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) start_microseconds: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) nonce_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct IntentBinding {
    pub(in crate::routine_work::runtime_adapter::production::custody) intent_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) node_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) plan_order: usize,
    pub(in crate::routine_work::runtime_adapter::production::custody) program_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct ChildLease {
    pub(in crate::routine_work::runtime_adapter::production::custody) process_id: i32,
    pub(in crate::routine_work::runtime_adapter::production::custody) process_group_id: i32,
    pub(in crate::routine_work::runtime_adapter::production::custody) executable_sha256: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) executable_device: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) executable_inode: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) intent: IntentBinding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct LaunchEntryIdentity {
    pub(in crate::routine_work::runtime_adapter::production::custody) device: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) inode: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) mode: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody) owner: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody) links: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) length: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct LaunchStageRecord {
    pub(in crate::routine_work::runtime_adapter::production::custody) directory:
        LaunchEntryIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) program: LaunchEntryIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) marker: LaunchEntryIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) seal: LaunchEntryIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) program_sha256: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) intent: IntentBinding,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct TerminalRecord {
    pub(in crate::routine_work::runtime_adapter::production::custody) state: AttemptState,
    pub(in crate::routine_work::runtime_adapter::production::custody) result_sha256: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) artifacts:
        BTreeMap<String, String>,
    pub(in crate::routine_work::runtime_adapter::production::custody) process_cleanup:
        CleanupEvidence,
    pub(in crate::routine_work::runtime_adapter::production::custody) staged_cleanup:
        CleanupEvidence,
    pub(in crate::routine_work::runtime_adapter::production::custody) prior_head_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::routine_work::runtime_adapter::production::custody) failure_evidence:
        Option<ReservationFailureEvidence>,
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
    pub(in crate::routine_work::runtime_adapter::production::custody) inner: supported::FileLedger,
}
