const MAX_TASKS: usize = 1_024;
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationError {
    code: &'static str,
}

impl EvaluationError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code)
    }
}

impl std::error::Error for EvaluationError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    Regular,
    Directory,
    Symlink,
    Special,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BoundInput {
    relative_path: String,
    digest_sha256: String,
    byte_length: u64,
    link_count: u64,
    kind: InputKind,
}

impl BoundInput {
    pub fn observed(
        relative_path: impl Into<String>,
        digest_sha256: impl Into<String>,
        byte_length: u64,
        link_count: u64,
        kind: InputKind,
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            digest_sha256: digest_sha256.into(),
            byte_length,
            link_count,
            kind,
        }
    }

    pub fn regular(
        relative_path: impl Into<String>,
        digest_sha256: impl Into<String>,
        byte_length: u64,
    ) -> Self {
        Self::observed(
            relative_path,
            digest_sha256,
            byte_length,
            1,
            InputKind::Regular,
        )
    }

    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub fn digest_sha256(&self) -> &str {
        &self.digest_sha256
    }

    fn findings(&self, prefix: &str) -> Vec<String> {
        let mut findings = Vec::new();
        if !safe_relative_path(&self.relative_path) {
            findings.push(format!("{prefix}-unsafe-path"));
        }
        if !valid_sha256(&self.digest_sha256) {
            findings.push(format!("{prefix}-invalid-digest"));
        }
        if self.byte_length == 0 || self.byte_length > MAX_INPUT_BYTES {
            findings.push(format!("{prefix}-size-out-of-bounds"));
        }
        if self.kind != InputKind::Regular {
            findings.push(format!("{prefix}-non-regular-input"));
        }
        if self.link_count != 1 {
            findings.push(format!("{prefix}-hardlink-rejected"));
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{:?}",
            self.relative_path, self.digest_sha256, self.byte_length, self.link_count, self.kind
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PerturbationControl {
    Verbosity,
    ProofArtifact,
    ReceiptProduction,
    TestManipulation,
    ScoreOnly,
}

impl PerturbationControl {
    pub const REQUIRED: [Self; 5] = [
        Self::Verbosity,
        Self::ProofArtifact,
        Self::ReceiptProduction,
        Self::TestManipulation,
        Self::ScoreOnly,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Verbosity => "verbosity",
            Self::ProofArtifact => "proof-artifact",
            Self::ReceiptProduction => "receipt-production",
            Self::TestManipulation => "test-manipulation",
            Self::ScoreOnly => "score-only",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationTask {
    task_id: String,
    requirement_id: String,
    behavior_id: String,
    fixture_id: String,
    dataset: BoundInput,
    dataset_provenance_sha256: String,
    known_training_corpus_sha256s: BTreeSet<String>,
    scorer_id: String,
    scorer_digest_sha256: String,
    perturbation_controls: BTreeSet<PerturbationControl>,
    representative: bool,
    data_controls: EvaluationDataControls,
}

/// Auditable data-quality facts used to reject leakage and ambiguous labels.
///
/// These are part of the immutable spec digest. They are not grader results and
/// cannot be supplied after an evaluation has run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationDataControls {
    objective: String,
    success_criterion: String,
    failure_criterion: String,
    split_id: String,
    training_split_ids: BTreeSet<String>,
    semantic_fingerprint_sha256: String,
    near_duplicate_group_sha256: String,
    known_training_fingerprint_sha256s: BTreeSet<String>,
    declared_label: String,
    verified_label: String,
    sampled_population: String,
    target_population: String,
}
