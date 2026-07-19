#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationEventKind {
    ExecutionReserved,
    ExecutionPublished,
    ExecutionInterrupted,
    RecoveryRequired,
    ExecutionTerminal,
    ReviewIssued,
    ReviewConsumed,
}

/// A deliberately closed event shape. It carries correlation and bounded
/// outcome facts, but has no claim, readiness, release, acceptance, or
/// completion fields and cannot authorize any transition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrivacySafeEvaluationEvent {
    pub schema_version: String,
    pub event_kind: EvaluationEventKind,
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub session_id: String,
    pub run_sha256: Option<String>,
    pub causal_code: Option<String>,
}

pub struct PrivacySafeEvaluationEventRecord {
    pub event_kind: EvaluationEventKind,
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub session_id: String,
    pub run_sha256: Option<String>,
    pub causal_code: Option<String>,
}

impl PrivacySafeEvaluationEvent {
    pub fn new(record: PrivacySafeEvaluationEventRecord) -> Result<Self, EvaluationError> {
        let PrivacySafeEvaluationEventRecord {
            event_kind,
            live_context_id,
            candidate_id,
            spec_sha256,
            session_id,
            run_sha256,
            causal_code,
        } = record;
        let event = Self {
            schema_version: "PrivacySafeEvaluationEvent-v1".to_owned(),
            event_kind,
            live_context_id,
            candidate_id,
            spec_sha256,
            session_id,
            run_sha256,
            causal_code,
        };
        if !super::valid_sha256(&event.live_context_id)
            || !super::valid_sha256(&event.candidate_id)
            || !super::valid_sha256(&event.spec_sha256)
            || !super::valid_sha256(&event.session_id)
            || event
                .run_sha256
                .as_deref()
                .is_some_and(|value| !super::valid_sha256(value))
            || event
                .causal_code
                .as_deref()
                .is_some_and(|value| !super::valid_identifier(value))
        {
            return Err(EvaluationError::new("evaluation-event-projection-invalid"));
        }
        Ok(event)
    }

    pub fn canonical_sha256(&self) -> String {
        canonical_sha256(self)
    }
}

pub(crate) fn sensitive_text(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    [
        "api_key",
        "api-key",
        "authorization:",
        "bearer ",
        "private_key",
        "client_secret",
        "access_token",
        "secret=",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn canonical_sha256(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("canonical evaluation record serializes");
    sha256(&bytes)
}
