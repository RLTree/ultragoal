use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct RootIdentity {
    pub(in crate::routine_work::runtime_adapter::production::custody) device: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) inode: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) owner: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody) mode: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct FileIdentity {
    pub(in crate::routine_work::runtime_adapter::production::custody) device: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) inode: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) owner: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody) mode: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody) links: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) length: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) changed_seconds: i64,
    pub(in crate::routine_work::runtime_adapter::production::custody) changed_nanos: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct ProtocolRecord {
    pub(in crate::routine_work::runtime_adapter::production::custody) binding: AuthorityBinding,
    pub(in crate::routine_work::runtime_adapter::production::custody) request_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) grant_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) recovery_marker: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) state: AttemptState,
    pub(in crate::routine_work::runtime_adapter::production::custody) owner: OwnerLease,
    pub(in crate::routine_work::runtime_adapter::production::custody) child: Option<ChildLease>,
    pub(in crate::routine_work::runtime_adapter::production::custody) launch_stage:
        Option<LaunchStageRecord>,
    pub(in crate::routine_work::runtime_adapter::production::custody) intents: Vec<IntentBinding>,
    pub(in crate::routine_work::runtime_adapter::production::custody) next_intent: usize,
    pub(in crate::routine_work::runtime_adapter::production::custody) issued_tick: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) expires_tick: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) recovery_deadline_tick: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) output_journal:
        OutputProvisionJournal,
    pub(in crate::routine_work::runtime_adapter::production::custody) terminal:
        Option<TerminalRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(in crate::routine_work::runtime_adapter::production::custody) failure_evidence:
        Vec<ReservationFailureEvidence>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct Payload {
    pub(in crate::routine_work::runtime_adapter::production::custody) schema_version: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) authority_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) key_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) root_identity: RootIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) lock_identity: FileIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) generation: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) previous_head_sha256: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) last_tick: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) attempts:
        BTreeMap<String, ProtocolRecord>,
    pub(in crate::routine_work::runtime_adapter::production::custody) effects:
        BTreeMap<String, String>,
    pub(in crate::routine_work::runtime_adapter::production::custody) consumed_grants:
        BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct Envelope {
    pub(in crate::routine_work::runtime_adapter::production::custody) payload: Payload,
    pub(in crate::routine_work::runtime_adapter::production::custody) hmac_sha256: String,
}

pub(in crate::routine_work::runtime_adapter::production::custody) struct LedgerKey(
    pub(in crate::routine_work::runtime_adapter::production::custody) [u8; KEY_BYTES],
);

pub(in crate::routine_work::runtime_adapter::production::custody) struct Store {
    pub(in crate::routine_work::runtime_adapter::production::custody) requested_root: PathBuf,
    pub(in crate::routine_work::runtime_adapter::production::custody) canonical_root: PathBuf,
    pub(in crate::routine_work::runtime_adapter::production::custody) directory: Arc<File>,
    pub(in crate::routine_work::runtime_adapter::production::custody) identity: RootIdentity,
}

pub(in crate::routine_work::runtime_adapter::production::custody) struct ProcessLock(
    pub(in crate::routine_work::runtime_adapter::production::custody) File,
);

pub(in crate::routine_work::runtime_adapter::production::custody) enum StatePublication {
    Committed,
    Precommit,
    Ambiguous,
}

#[derive(Clone)]
pub(in crate::routine_work::runtime_adapter::production::custody) struct LocalHead {
    pub(in crate::routine_work::runtime_adapter::production::custody) generation: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody) head_sha256: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) state_identity: FileIdentity,
}

pub(in crate::routine_work::runtime_adapter::production::custody) struct FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody) store: Store,
    pub(in crate::routine_work::runtime_adapter::production::custody) key_identity: FileIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) lock_identity: FileIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody) key_id: String,
    pub(in crate::routine_work::runtime_adapter::production::custody) authority_id: String,
}
