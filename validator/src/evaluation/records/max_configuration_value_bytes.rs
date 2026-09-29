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
    #[cfg(test)]
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
