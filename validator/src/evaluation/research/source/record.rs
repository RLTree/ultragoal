pub struct ResearchSourceRecordDefinition {
    pub source_id: String,
    pub publisher: String,
    pub url: String,
    pub source_class: ResearchSourceClass,
    pub checked_date: String,
    pub observed_at_epoch_seconds: u64,
    pub verified_source_facts: Vec<VerifiedSourceFact>,
    pub binding_product_requirements: Vec<BindingProductRequirement>,
    pub advisory_practices: Vec<AdvisoryPractice>,
    pub experimental_hypotheses: Vec<ExperimentalHypothesis>,
    pub rejected_recommendations: Vec<RejectedRecommendation>,
    pub limitations: Vec<String>,
    pub mapped_law_ids: BTreeSet<String>,
    pub supports_proposal_ids: BTreeSet<String>,
}

impl ResearchSourceRecord {
    pub fn new(definition: ResearchSourceRecordDefinition) -> Result<Self, EvaluationError> {
        let ResearchSourceRecordDefinition {
            source_id,
            publisher,
            url,
            source_class,
            checked_date,
            observed_at_epoch_seconds,
            verified_source_facts,
            binding_product_requirements,
            advisory_practices,
            experimental_hypotheses,
            rejected_recommendations,
            limitations,
            mapped_law_ids,
            supports_proposal_ids,
        } = definition;
        let valid_until_epoch_seconds = observed_at_epoch_seconds
            .checked_add(freshness_window_seconds(&verified_source_facts))
            .ok_or_else(invalid_source)?;
        let value = Self {
            schema_version: "ResearchSourceRecord-v1".to_owned(),
            source_id,
            publisher,
            url,
            source_class,
            checked_date,
            observed_at_epoch_seconds,
            valid_until_epoch_seconds,
            verified_source_facts,
            binding_product_requirements,
            advisory_practices,
            experimental_hypotheses,
            rejected_recommendations,
            limitations,
            mapped_law_ids,
            supports_proposal_ids,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn source_class(&self) -> ResearchSourceClass {
        self.source_class
    }

    pub fn mapped_law_ids(&self) -> &BTreeSet<String> {
        &self.mapped_law_ids
    }

    pub fn supports_proposal_ids(&self) -> &BTreeSet<String> {
        &self.supports_proposal_ids
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("typed research source record serializes")
    }

    fn validate(&self) -> Result<(), EvaluationError> {
        if self.schema_version != "ResearchSourceRecord-v1"
            || !valid_identifier(&self.source_id)
            || !valid_text(&self.publisher, 512)
            || !valid_https_url(&self.url)
            || !checked_date_matches_epoch(&self.checked_date, self.observed_at_epoch_seconds)
            || self.observed_at_epoch_seconds == 0
            || self
                .observed_at_epoch_seconds
                .checked_add(freshness_window_seconds(&self.verified_source_facts))
                != Some(self.valid_until_epoch_seconds)
            || !valid_nonempty_rows(&self.verified_source_facts)
            || !valid_nonempty_rows(&self.binding_product_requirements)
            || !valid_nonempty_rows(&self.advisory_practices)
            || !valid_nonempty_rows(&self.experimental_hypotheses)
            || !valid_nonempty_rows(&self.rejected_recommendations)
            || !valid_text_list(&self.limitations)
            || !valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
            || !valid_identifier_set(&self.supports_proposal_ids, MAX_PROPOSALS)
        {
            return Err(invalid_source());
        }

        let mut row_ids = BTreeSet::new();
        let mut statements = BTreeSet::new();
        for fact in &self.verified_source_facts {
            if !fact.valid()
                || fact.source_id != self.source_id
                || !valid_source_locator(&fact.source_locator, &self.url)
                || !law_subset(&fact.mapped_law_ids, &self.mapped_law_ids)
                || !row_ids.insert(fact.fact_id.as_str())
                || !statements.insert(fact.statement.as_str())
            {
                return Err(invalid_source());
            }
        }
        for requirement in &self.binding_product_requirements {
            if !requirement.valid()
                || !self.mapped_law_ids.contains(&requirement.authority_law_id)
                || !row_ids.insert(requirement.requirement_id.as_str())
                || !statements.insert(requirement.statement.as_str())
            {
                return Err(invalid_source());
            }
        }
        for practice in &self.advisory_practices {
            if !practice.valid()
                || !valid_record_sourced_row(
                    &practice.practice_id,
                    &practice.statement,
                    &practice.source_id,
                    &practice.source_locator,
                    &practice.mapped_law_ids,
                    self,
                    (&mut row_ids, &mut statements),
                )
            {
                return Err(invalid_source());
            }
        }
        for hypothesis in &self.experimental_hypotheses {
            if !hypothesis.valid()
                || !valid_record_sourced_row(
                    &hypothesis.hypothesis_id,
                    &hypothesis.statement,
                    &hypothesis.source_id,
                    &hypothesis.source_locator,
                    &hypothesis.mapped_law_ids,
                    self,
                    (&mut row_ids, &mut statements),
                )
            {
                return Err(invalid_source());
            }
        }
        for recommendation in &self.rejected_recommendations {
            if !recommendation.valid()
                || !valid_record_sourced_row(
                    &recommendation.recommendation_id,
                    &recommendation.statement,
                    &recommendation.source_id,
                    &recommendation.source_locator,
                    &recommendation.mapped_law_ids,
                    self,
                    (&mut row_ids, &mut statements),
                )
            {
                return Err(invalid_source());
            }
        }
        Ok(())
    }

    fn is_current_at(&self, current_epoch_seconds: u64) -> bool {
        self.observed_at_epoch_seconds <= current_epoch_seconds
            && self.valid_until_epoch_seconds >= current_epoch_seconds
    }

    fn requires_current_primary_source(&self) -> bool {
        !self.verified_source_facts.is_empty()
    }

    fn has_externally_untrusted_stable_fact(&self) -> bool {
        self.verified_source_facts
            .iter()
            .any(|fact| fact.temporal_scope == FactTemporalScope::Stable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResearchSource {
    snapshot: BoundInput,
    record: ResearchSourceRecord,
    #[serde(skip_serializing)]
    record_bytes: Vec<u8>,
    #[serde(skip_serializing)]
    authority_binding: ResearchSourceAuthorityBinding,
    #[serde(skip_serializing)]
    mutation_revision: u64,
}
