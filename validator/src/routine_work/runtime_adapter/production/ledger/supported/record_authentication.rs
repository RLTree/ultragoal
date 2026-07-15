use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) owner: u32,
    pub(crate) mode: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FileIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) owner: u32,
    pub(crate) mode: u32,
    pub(crate) links: u64,
    pub(crate) length: u64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProtocolRecord {
    pub(crate) binding: AuthorityBinding,
    pub(crate) request_id: String,
    pub(crate) grant_id: String,
    pub(crate) recovery_marker: String,
    pub(crate) recovery_for: Option<String>,
    pub(crate) state: AttemptState,
    pub(crate) reuse_only: bool,
    pub(crate) issued_tick: u64,
    pub(crate) expires_tick: u64,
    pub(crate) recovery_deadline_tick: u64,
    pub(crate) artifacts: BTreeMap<String, String>,
    pub(crate) output_journal: OutputProvisionJournal,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) failure_evidence: Vec<ReservationFailureEvidence>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    pub(crate) schema_version: String,
    pub(crate) authority_id: String,
    pub(crate) key_id: String,
    pub(crate) root_identity: RootIdentity,
    pub(crate) lock_identity: FileIdentity,
    pub(crate) generation: u64,
    pub(crate) previous_head_sha256: String,
    pub(crate) last_tick: u64,
    pub(crate) protocols: BTreeMap<String, ProtocolRecord>,
    pub(crate) effects: BTreeMap<String, String>,
    pub(crate) consumed_grants: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    pub(crate) payload: Payload,
    pub(crate) hmac_sha256: String,
}

pub(crate) struct LedgerKey(pub(crate) [u8; KEY_BYTES]);

pub(crate) struct Store {
    pub(crate) requested_root: PathBuf,
    pub(crate) canonical_root: PathBuf,
    pub(crate) directory: Arc<File>,
    pub(crate) identity: RootIdentity,
}

pub(crate) struct ProcessLock(pub(crate) File);

#[derive(Clone)]
pub(crate) struct LocalHead {
    pub(crate) generation: u64,
    pub(crate) head_sha256: String,
    pub(crate) state_identity: FileIdentity,
}

pub(crate) struct FileLedger {
    pub(crate) store: Store,
    pub(crate) key_identity: FileIdentity,
    pub(crate) lock_identity: FileIdentity,
    pub(crate) key_id: String,
    pub(crate) authority_id: String,
    pub(crate) local: Mutex<LocalHead>,
}
