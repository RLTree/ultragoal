const STATE_NAME: &str = "execution.state";
const ANCHOR_NAME: &str = "execution.anchor.journal";
const LOCK_NAME: &str = "execution.lock";
const MAX_LEDGER_BYTES: u64 = 1024 * 1024;
const MAX_ANCHOR_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ANCHOR_RECORD_BYTES: usize = 1024 * 1024;
const ANCHOR_GENESIS: &[u8] = b"evaluation-anchor-journal-genesis";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EvaluationExecutionBinding {
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub task_set_sha256: String,
    pub execution_session_id: String,
    pub executable_set_sha256: String,
    pub artifact_root_sha256: String,
}

pub struct EvaluationExecutionBindingRequest {
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub task_set_sha256: String,
    pub execution_session_id: String,
    pub executable_set_sha256: String,
    pub artifact_root_sha256: String,
}

impl EvaluationExecutionBinding {
    pub fn new(request: EvaluationExecutionBindingRequest) -> Result<Self, EvaluationLedgerError> {
        let EvaluationExecutionBindingRequest {
            live_context_id,
            candidate_id,
            spec_sha256,
            task_set_sha256,
            execution_session_id,
            executable_set_sha256,
            artifact_root_sha256,
        } = request;
        let value = Self {
            live_context_id,
            candidate_id,
            spec_sha256,
            task_set_sha256,
            execution_session_id,
            executable_set_sha256,
            artifact_root_sha256,
        };
        if [
            value.live_context_id.as_str(),
            value.candidate_id.as_str(),
            value.spec_sha256.as_str(),
            value.task_set_sha256.as_str(),
            value.execution_session_id.as_str(),
            value.executable_set_sha256.as_str(),
            value.artifact_root_sha256.as_str(),
        ]
        .iter()
        .any(|item| !super::valid_sha256(item))
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-ledger-binding-invalid",
            ));
        }
        Ok(value)
    }

    pub(crate) fn digest(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("execution binding serializes"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvaluationLedgerState {
    Initialized,
    Reserved,
    Published {
        run_sha256: String,
        artifact_set_sha256: String,
    },
    Interrupted {
        causal_code: String,
    },
    RecoveryRequired {
        causal_code: String,
    },
    Terminal {
        run_sha256: String,
        artifact_set_sha256: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationLedgerError {
    code: &'static str,
}

impl EvaluationLedgerError {
    pub(crate) const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for EvaluationLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for EvaluationLedgerError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionTerminalProof {
    pub binding: EvaluationExecutionBinding,
    pub run_sha256: String,
    pub artifact_set_sha256: String,
    pub ledger_head_sha256: String,
}

#[derive(Debug)]
pub struct FileEvaluationExecutionLedger {
    root_path: PathBuf,
    root: File,
    root_identity: FileIdentity,
    lock: File,
    lock_identity: FileIdentity,
    anchor: File,
    anchor_authority: FileAuthorityIdentity,
    key: [u8; 32],
    key_id: String,
    binding: EvaluationExecutionBinding,
    expected_head: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FileIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) links: u64,
    pub(super) length: u64,
    pub(super) changed_seconds: i64,
    pub(super) changed_nanos: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct FileAuthorityIdentity {
    pub(super) device: u64,
    pub(super) inode: u64,
    pub(super) mode: u32,
    pub(super) links: u64,
}

impl FileIdentity {
    pub(super) const fn authority(self) -> FileAuthorityIdentity {
        FileAuthorityIdentity {
            device: self.device,
            inode: self.inode,
            mode: self.mode,
            links: self.links,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SnapshotCore {
    schema_version: String,
    generation: u64,
    previous_head_sha256: String,
    key_id: String,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: EvaluationExecutionBinding,
    state: EvaluationLedgerState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SnapshotPayload {
    core: SnapshotCore,
    anchor_observation: FileIdentity,
    anchor_length: u64,
    anchor_head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedSnapshot {
    payload: SnapshotPayload,
    mac_sha256: String,
    head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AnchorRecordPayload {
    schema_version: String,
    prior_anchor_head_sha256: String,
    core: SnapshotCore,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedAnchorRecord {
    payload: AnchorRecordPayload,
    mac_sha256: String,
    head_sha256: String,
}
