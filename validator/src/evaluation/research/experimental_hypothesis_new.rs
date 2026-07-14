impl ExperimentalHypothesis {
    pub fn new(
        hypothesis_id: impl Into<String>,
        statement: impl Into<String>,
        source_id: impl Into<String>,
        source_locator: impl Into<String>,
        mapped_law_ids: BTreeSet<String>,
        falsification_test: impl Into<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            hypothesis_id: hypothesis_id.into(),
            statement: statement.into(),
            source_id: source_id.into(),
            source_locator: source_locator.into(),
            mapped_law_ids,
            falsification_test: falsification_test.into(),
        };
        if !value.valid() {
            return Err(invalid_source());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_sourced_row(
            &self.hypothesis_id,
            &self.statement,
            &self.source_id,
            &self.source_locator,
            &self.mapped_law_ids,
        ) && valid_text(&self.falsification_test, MAX_TEXT_BYTES)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RejectedRecommendation {
    recommendation_id: String,
    statement: String,
    source_id: String,
    source_locator: String,
    mapped_law_ids: BTreeSet<String>,
    rejection_reason: String,
}

impl RejectedRecommendation {
    pub fn new(
        recommendation_id: impl Into<String>,
        statement: impl Into<String>,
        source_id: impl Into<String>,
        source_locator: impl Into<String>,
        mapped_law_ids: BTreeSet<String>,
        rejection_reason: impl Into<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            recommendation_id: recommendation_id.into(),
            statement: statement.into(),
            source_id: source_id.into(),
            source_locator: source_locator.into(),
            mapped_law_ids,
            rejection_reason: rejection_reason.into(),
        };
        if !value.valid() {
            return Err(invalid_source());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_sourced_row(
            &self.recommendation_id,
            &self.statement,
            &self.source_id,
            &self.source_locator,
            &self.mapped_law_ids,
        ) && valid_text(&self.rejection_reason, MAX_TEXT_BYTES)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchSourceRecord {
    schema_version: String,
    source_id: String,
    publisher: String,
    url: String,
    source_class: ResearchSourceClass,
    checked_date: String,
    observed_at_epoch_seconds: u64,
    valid_until_epoch_seconds: u64,
    verified_source_facts: Vec<VerifiedSourceFact>,
    binding_product_requirements: Vec<BindingProductRequirement>,
    advisory_practices: Vec<AdvisoryPractice>,
    experimental_hypotheses: Vec<ExperimentalHypothesis>,
    rejected_recommendations: Vec<RejectedRecommendation>,
    limitations: Vec<String>,
    mapped_law_ids: BTreeSet<String>,
    supports_proposal_ids: BTreeSet<String>,
}
