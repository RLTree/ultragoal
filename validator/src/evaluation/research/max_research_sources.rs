const MAX_RESEARCH_SOURCES: usize = 128;
const MAX_PROPOSALS: usize = 32;
const MAX_CLASSIFICATION_ROWS: usize = 128;
const MAX_SOURCE_RECORD_BYTES: usize = 256 * 1024;
const MAX_TEXT_BYTES: usize = 4 * 1024;
const MAX_URL_BYTES: usize = 2 * 1024;
const SECONDS_PER_DAY: u64 = 86_400;
const MUTABLE_CLAIM_FRESHNESS_SECONDS: u64 = 30 * SECONDS_PER_DAY;
const ROOT_RESEARCH_DECISION_ID: &str = "OD-RESEARCH-ADOPTION";
const ROOT_RESEARCH_REVIEWER_ROLE: &str = "ultra-root";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NoAuthorityEffect {
    None,
}

impl NoAuthorityEffect {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchSourceClass {
    PrimarySpecification,
    PrimaryMeasurement,
    PeerReviewedResearch,
    VendorDocumentation,
}

impl ResearchSourceClass {
    const fn supports_current_capability_decision(self) -> bool {
        matches!(self, Self::PrimarySpecification | Self::PrimaryMeasurement)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactTemporalScope {
    Stable,
    MutableCapability,
    VersionClaim,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedSourceFact {
    fact_id: String,
    statement: String,
    source_id: String,
    source_locator: String,
    mapped_law_ids: BTreeSet<String>,
    temporal_scope: FactTemporalScope,
}

impl VerifiedSourceFact {
    pub fn new(
        fact_id: impl Into<String>,
        statement: impl Into<String>,
        source_id: impl Into<String>,
        source_locator: impl Into<String>,
        mapped_law_ids: BTreeSet<String>,
        temporal_scope: FactTemporalScope,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            fact_id: fact_id.into(),
            statement: statement.into(),
            source_id: source_id.into(),
            source_locator: source_locator.into(),
            mapped_law_ids,
            temporal_scope,
        };
        if !value.valid() {
            return Err(invalid_source());
        }
        Ok(value)
    }

    pub fn temporal_scope(&self) -> FactTemporalScope {
        self.temporal_scope
    }

    fn valid(&self) -> bool {
        valid_identifier(&self.fact_id)
            && valid_text(&self.statement, MAX_TEXT_BYTES)
            && valid_identifier(&self.source_id)
            && valid_https_locator(&self.source_locator)
            && valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BindingProductRequirement {
    requirement_id: String,
    statement: String,
    authority_law_id: String,
}

impl BindingProductRequirement {
    pub fn new(
        requirement_id: impl Into<String>,
        statement: impl Into<String>,
        authority_law_id: impl Into<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            requirement_id: requirement_id.into(),
            statement: statement.into(),
            authority_law_id: authority_law_id.into(),
        };
        if !value.valid() {
            return Err(invalid_source());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_identifier(&self.requirement_id)
            && valid_text(&self.statement, MAX_TEXT_BYTES)
            && valid_identifier(&self.authority_law_id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdvisoryPractice {
    practice_id: String,
    statement: String,
    source_id: String,
    source_locator: String,
    mapped_law_ids: BTreeSet<String>,
}

impl AdvisoryPractice {
    pub fn new(
        practice_id: impl Into<String>,
        statement: impl Into<String>,
        source_id: impl Into<String>,
        source_locator: impl Into<String>,
        mapped_law_ids: BTreeSet<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            practice_id: practice_id.into(),
            statement: statement.into(),
            source_id: source_id.into(),
            source_locator: source_locator.into(),
            mapped_law_ids,
        };
        if !value.valid() {
            return Err(invalid_source());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_sourced_row(
            &self.practice_id,
            &self.statement,
            &self.source_id,
            &self.source_locator,
            &self.mapped_law_ids,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentalHypothesis {
    hypothesis_id: String,
    statement: String,
    source_id: String,
    source_locator: String,
    mapped_law_ids: BTreeSet<String>,
    falsification_test: String,
}
