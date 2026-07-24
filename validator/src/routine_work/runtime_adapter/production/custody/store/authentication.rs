use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct RootIdentity
{
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) device: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) inode: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) owner: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) mode: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct FileIdentity
{
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) device: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) inode: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) owner: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) mode: u32,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) links: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) length: u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) changed_seconds:
        i64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) changed_nanos:
        i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct ProtocolRecord
{
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) binding:
        AuthorityBinding,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) request_id:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) grant_id:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) recovery_marker:
        String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) predecessor_continuation:
        Option<String>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) state:
        AttemptState,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) owner:
        OwnerLease,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) child:
        Option<ChildLease>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) launch_stage:
        Option<LaunchStageRecord>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) intents:
        Vec<IntentBinding>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) next_intent:
        usize,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) issued_tick:
        u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) expires_tick:
        u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) recovery_deadline_tick:
        u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) output_journal:
        OutputProvisionJournal,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) terminal:
        Option<TerminalRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) publication_ambiguity:
        Option<PublicationAmbiguity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) failure_evidence:
        Vec<ReservationFailureEvidence>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct Payload {
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) schema_version:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) authority_id:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) key_id:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) root_identity:
        RootIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) lock_identity:
        FileIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) generation:
        u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) previous_head_sha256:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) last_tick:
        u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) attempts:
        BTreeMap<String, ProtocolRecord>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) effects:
        BTreeMap<String, String>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) consumed_grants:
        BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct Envelope
{
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) payload:
        Payload,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) hmac_sha256:
        String,
}

pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct LedgerKey(
    [u8; KEY_BYTES],
);

impl LedgerKey {
    pub(super) fn new(bytes: [u8; KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub(super) fn bytes(&self) -> &[u8; KEY_BYTES] {
        &self.0
    }
}

pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct Store {
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) requested_root:
        PathBuf,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) canonical_root:
        PathBuf,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) directory:
        Arc<File>,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) identity:
        RootIdentity,
}

pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) struct ProcessLock(
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) File,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) enum StatePublication
{
    Committed,
    Precommit,
    Ambiguous,
}

#[derive(Clone)]
pub(in crate::routine_work::runtime_adapter::production::custody::store) struct LocalHead {
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) generation:
        u64,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) head_sha256:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) state_identity:
        FileIdentity,
}

impl LocalHead {
    pub(in crate::routine_work::runtime_adapter::production::custody::store) fn head_sha256(
        &self,
    ) -> &str {
        &self.head_sha256
    }
}

pub(in crate::routine_work::runtime_adapter::production::custody::store) struct FileLedger {
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) store:
        Store,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) key_identity:
        FileIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) lock_identity:
        FileIdentity,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) key_id:
        String,
    pub(in crate::routine_work::runtime_adapter::production::custody::store::supported) authority_id:
        String,
}
