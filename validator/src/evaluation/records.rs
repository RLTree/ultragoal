use super::{EvaluationError, EvaluationRun, EvaluationTaskResult, FailureCase, PromotionDecision};
use crate::fixture_scheduler::FixtureExecutionRecord;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const MAX_CONFIGURATION_VALUE_BYTES: usize = 512;
const MAX_TOOLS: usize = 64;

/// Provenance classes that can expose configuration. Prompt text is
/// intentionally not a member of this enum.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationSource {
    RuntimeApi,
    ToolApi,
    SignedReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "exposure", rename_all = "snake_case")]
pub enum ConfigurationExposure {
    Exposed {
        value: String,
        source: ConfigurationSource,
        evidence_sha256: String,
    },
    Unknown,
}

impl ConfigurationExposure {
    pub fn exposed(
        value: impl Into<String>,
        source: ConfigurationSource,
        evidence_sha256: impl Into<String>,
    ) -> Result<Self, EvaluationError> {
        let value = value.into();
        let evidence_sha256 = evidence_sha256.into();
        if value.is_empty()
            || value.len() > MAX_CONFIGURATION_VALUE_BYTES
            || value.chars().any(char::is_control)
            || sensitive_text(value.as_bytes())
            || !super::valid_sha256(&evidence_sha256)
        {
            return Err(EvaluationError::new(
                "evaluation-runtime-provenance-invalid",
            ));
        }
        Ok(Self::Exposed {
            value,
            source,
            evidence_sha256,
        })
    }

    pub const fn unknown() -> Self {
        Self::Unknown
    }
}

/// Vendor-neutral runtime metadata. Every field is either supported by an
/// exposed runtime/tool/receipt provenance or explicitly unknown.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeConfiguration {
    pub model: ConfigurationExposure,
    pub runtime: ConfigurationExposure,
    pub tools: BTreeMap<String, ConfigurationExposure>,
}

impl RuntimeConfiguration {
    pub fn new(
        model: ConfigurationExposure,
        runtime: ConfigurationExposure,
        tools: BTreeMap<String, ConfigurationExposure>,
    ) -> Result<Self, EvaluationError> {
        if tools.len() > MAX_TOOLS || tools.keys().any(|key| !super::valid_identifier(key)) {
            return Err(EvaluationError::new(
                "evaluation-runtime-provenance-invalid",
            ));
        }
        Ok(Self {
            model,
            runtime,
            tools,
        })
    }

    pub fn all_unknown() -> Self {
        Self {
            model: ConfigurationExposure::Unknown,
            runtime: ConfigurationExposure::Unknown,
            tools: BTreeMap::new(),
        }
    }

    /// Prompt text is an untrusted workload input, never runtime provenance.
    pub fn from_prompt_text(_prompt: &str) -> Result<Self, EvaluationError> {
        Err(EvaluationError::new(
            "evaluation-prompt-derived-runtime-metadata-refused",
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEvaluationRun {
    pub schema_version: &'static str,
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_id: String,
    pub spec_sha256: String,
    pub task_set_sha256: String,
    pub execution_session_id: String,
    pub run_sha256: String,
    pub runtime_configuration: RuntimeConfiguration,
    pub fixture_execution_set_sha256: String,
    pub results: Vec<EvaluationTaskResult>,
}

impl CanonicalEvaluationRun {
    pub(crate) fn from_parts(
        run: &EvaluationRun,
        runtime_configuration: RuntimeConfiguration,
        records: &[FixtureExecutionRecord],
    ) -> Self {
        let rows = records
            .iter()
            .map(FixtureExecutionRecord::record_sha256)
            .collect::<Vec<_>>()
            .join("\n");
        Self {
            schema_version: "CanonicalEvaluationRun-v1",
            live_context_id: run.live_context_id.clone(),
            candidate_id: run.candidate_id.clone(),
            spec_id: run.spec_id.clone(),
            spec_sha256: run.spec_sha256.clone(),
            task_set_sha256: run.task_set_sha256.clone(),
            execution_session_id: run.execution_session_id.clone(),
            run_sha256: run.run_sha256.clone(),
            runtime_configuration,
            fixture_execution_set_sha256: sha256(rows.as_bytes()),
            results: run.results.clone(),
        }
    }

    pub fn canonical_sha256(&self) -> String {
        canonical_sha256(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEvaluationFailure {
    pub schema_version: &'static str,
    pub live_context_id: String,
    pub candidate_id: String,
    pub run_sha256: String,
    pub task_id: String,
    pub fixture_id: String,
    pub causal_code: String,
    pub artifact_digest_sha256: String,
}

impl From<&FailureCase> for CanonicalEvaluationFailure {
    fn from(value: &FailureCase) -> Self {
        Self {
            schema_version: "CanonicalEvaluationFailure-v1",
            live_context_id: value.live_context_id.clone(),
            candidate_id: value.candidate_id.clone(),
            run_sha256: value.run_sha256.clone(),
            task_id: value.task_id.clone(),
            fixture_id: value.fixture_id.clone(),
            causal_code: value.causal_code.clone(),
            artifact_digest_sha256: value.artifact_digest_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CanonicalEvaluationReview {
    pub schema_version: &'static str,
    pub baseline_candidate_id: String,
    pub candidate_id: String,
    pub status: super::PromotionStatus,
    pub baseline_run_sha256: String,
    pub candidate_run_sha256: String,
    pub reasons: Vec<String>,
    pub maximum_effect: &'static str,
}

impl From<&PromotionDecision> for CanonicalEvaluationReview {
    fn from(value: &PromotionDecision) -> Self {
        Self {
            schema_version: "CanonicalEvaluationReview-v1",
            baseline_candidate_id: value.baseline_candidate_id.clone(),
            candidate_id: value.candidate_id.clone(),
            status: value.status,
            baseline_run_sha256: value.baseline_run_sha256.clone(),
            candidate_run_sha256: value.candidate_run_sha256.clone(),
            reasons: value.reasons.clone(),
            maximum_effect: "non_authoritative_improvement_candidate",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PrivacySafeEvaluationEvent {
    pub schema_version: &'static str,
    pub event_kind: EvaluationEventKind,
    pub live_context_id: String,
    pub candidate_id: String,
    pub spec_sha256: String,
    pub session_id: String,
    pub run_sha256: Option<String>,
    pub causal_code: Option<String>,
}

impl PrivacySafeEvaluationEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_kind: EvaluationEventKind,
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        spec_sha256: impl Into<String>,
        session_id: impl Into<String>,
        run_sha256: Option<String>,
        causal_code: Option<String>,
    ) -> Result<Self, EvaluationError> {
        let event = Self {
            schema_version: "PrivacySafeEvaluationEvent-v1",
            event_kind,
            live_context_id: live_context_id.into(),
            candidate_id: candidate_id.into(),
            spec_sha256: spec_sha256.into(),
            session_id: session_id.into(),
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
