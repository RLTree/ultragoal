const STATE_NAME: &str = "promotion-review.state";
const ANCHOR_NAME: &str = "promotion-review.anchor.journal";
const LOCK_NAME: &str = "promotion-review.lock";
const INITIAL_ANCHOR_NAME: &str = ".promotion-review.anchor.journal.initializing";
const INITIAL_STATE_NAME: &str = ".promotion-review.state.initializing";
const MAX_LEDGER_BYTES: u64 = 1024 * 1024;
const MAX_ANCHOR_JOURNAL_BYTES: u64 = 16 * 1024 * 1024;
const MAX_ANCHOR_RECORD_BYTES: usize = 1024 * 1024;
const ANCHOR_GENESIS: &[u8] = b"promotion-anchor-journal-genesis";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PromotionLedgerBinding {
    authority_id: String,
    reviewer_id: String,
    review_session_id: String,
    live_context_id: String,
    baseline_candidate_id: String,
    candidate_id: String,
    baseline_run_sha256: String,
    candidate_run_sha256: String,
    baseline_execution_session_id: String,
    candidate_execution_session_id: String,
    baseline_execution_head_sha256: String,
    candidate_execution_head_sha256: String,
}

impl PromotionLedgerBinding {
    pub(crate) fn from_terminal_proofs(
        authority_id: impl Into<String>,
        reviewer_id: impl Into<String>,
        review_session_id: impl Into<String>,
        baseline: ExecutionTerminalProof,
        candidate: ExecutionTerminalProof,
    ) -> Result<Self, PromotionLedgerError> {
        let value = Self {
            authority_id: authority_id.into(),
            reviewer_id: reviewer_id.into(),
            review_session_id: review_session_id.into(),
            live_context_id: candidate.binding.live_context_id.clone(),
            baseline_candidate_id: baseline.binding.candidate_id.clone(),
            candidate_id: candidate.binding.candidate_id.clone(),
            baseline_run_sha256: baseline.run_sha256,
            candidate_run_sha256: candidate.run_sha256,
            baseline_execution_session_id: baseline.binding.execution_session_id,
            candidate_execution_session_id: candidate.binding.execution_session_id,
            baseline_execution_head_sha256: baseline.ledger_head_sha256,
            candidate_execution_head_sha256: candidate.ledger_head_sha256,
        };
        if baseline.binding.live_context_id != value.live_context_id
            || value.baseline_candidate_id == value.candidate_id
            || !super::valid_identifier(&value.authority_id)
            || !super::valid_identifier(&value.reviewer_id)
            || value.authority_id == value.reviewer_id
            || !super::valid_sha256(&value.review_session_id)
            || value.review_session_id == value.baseline_execution_session_id
            || value.review_session_id == value.candidate_execution_session_id
            || value.baseline_execution_session_id == value.candidate_execution_session_id
            || [
                value.live_context_id.as_str(),
                value.baseline_candidate_id.as_str(),
                value.candidate_id.as_str(),
                value.baseline_run_sha256.as_str(),
                value.candidate_run_sha256.as_str(),
                value.baseline_execution_head_sha256.as_str(),
                value.candidate_execution_head_sha256.as_str(),
            ]
            .iter()
            .any(|item| !super::valid_sha256(item))
        {
            return Err(PromotionLedgerError::new(
                "promotion-ledger-binding-invalid",
            ));
        }
        Ok(value)
    }

    fn digest(&self) -> String {
        sha256(&serde_json::to_vec(self).expect("promotion binding serializes"))
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PromotionLedgerState {
    Ready,
    Issued {
        binding_sha256: String,
        attestation_sha256: String,
    },
    Consumed {
        binding_sha256: String,
        review_id: String,
        attestation_sha256: String,
    },
    RecoveryRequired {
        causal_code: String,
    },
}

/// The locked journal decision for an attempted one-shot review consumption.
/// A caller receives the causal losing state rather than treating a later read
/// as proof that it lost the same race.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PromotionConsumptionOutcome {
    Consumed,
    AlreadyConsumed { review_id: String },
    Refused { causal_code: &'static str },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromotionLedgerError {
    code: &'static str,
}

impl PromotionLedgerError {
    const fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for PromotionLedgerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for PromotionLedgerError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ReviewSnapshotCore {
    schema_version: String,
    generation: u64,
    previous_head_sha256: String,
    key_id: String,
    lock_identity: FileIdentity,
    anchor_authority: FileAuthorityIdentity,
    binding: PromotionLedgerBinding,
    state: PromotionLedgerState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReviewSnapshotPayload {
    core: ReviewSnapshotCore,
    anchor_observation: FileIdentity,
    anchor_length: u64,
    anchor_head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedReviewSnapshot {
    payload: ReviewSnapshotPayload,
    mac_sha256: String,
    head_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReviewAnchorRecordPayload {
    schema_version: String,
    prior_anchor_head_sha256: String,
    core: ReviewSnapshotCore,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatedReviewAnchorRecord {
    payload: ReviewAnchorRecordPayload,
    mac_sha256: String,
    head_sha256: String,
}

struct CurrentReviewSnapshot {
    snapshot: AuthenticatedReviewSnapshot,
    partial_tail_from: Option<u64>,
    observed_state_head_sha256: String,
    observed_anchor: FileIdentity,
}

#[derive(Debug)]
pub struct FilePromotionReviewLedger {
    root_path: PathBuf,
    root: File,
    root_identity: FileIdentity,
    lock: File,
    lock_identity: FileIdentity,
    anchor: File,
    anchor_authority: FileAuthorityIdentity,
    key: [u8; 32],
    key_id: String,
    binding: PromotionLedgerBinding,
    expected_head: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingReviewPublication {
    name: String,
    generation: u64,
    identity: FileIdentity,
}
