use super::{BoundInput, EvaluationError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

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

impl ResearchSourceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_id: impl Into<String>,
        publisher: impl Into<String>,
        url: impl Into<String>,
        source_class: ResearchSourceClass,
        checked_date: impl Into<String>,
        observed_at_epoch_seconds: u64,
        verified_source_facts: Vec<VerifiedSourceFact>,
        binding_product_requirements: Vec<BindingProductRequirement>,
        advisory_practices: Vec<AdvisoryPractice>,
        experimental_hypotheses: Vec<ExperimentalHypothesis>,
        rejected_recommendations: Vec<RejectedRecommendation>,
        limitations: Vec<String>,
        mapped_law_ids: BTreeSet<String>,
        supports_proposal_ids: BTreeSet<String>,
    ) -> Result<Self, EvaluationError> {
        let valid_until_epoch_seconds = observed_at_epoch_seconds
            .checked_add(freshness_window_seconds(&verified_source_facts))
            .ok_or_else(invalid_source)?;
        let value = Self {
            schema_version: "ResearchSourceRecord-v1".to_owned(),
            source_id: source_id.into(),
            publisher: publisher.into(),
            url: url.into(),
            source_class,
            checked_date: checked_date.into(),
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
                    &mut row_ids,
                    &mut statements,
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
                    &mut row_ids,
                    &mut statements,
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
                    &mut row_ids,
                    &mut statements,
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

/// Public research records are untrusted descriptions.  In particular, a
/// caller-controlled digest proves byte consistency, not who established the
/// source class, temporal policy, or observation time.  Keep that provenance
/// separate from the serialized record so recomputing canonical bytes cannot
/// mint review authority.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ResearchSourceAuthorityBinding {
    UnverifiedExternal,
    #[cfg(test)]
    TestRootAdopted {
        record_sha256: String,
        source_class: ResearchSourceClass,
        observed_at_epoch_seconds: u64,
        valid_until_epoch_seconds: u64,
    },
}

impl ResearchSourceAuthorityBinding {
    fn validates(&self, _record: &ResearchSourceRecord, _record_bytes: &[u8]) -> bool {
        match self {
            Self::UnverifiedExternal => false,
            #[cfg(test)]
            Self::TestRootAdopted {
                record_sha256,
                source_class,
                observed_at_epoch_seconds,
                valid_until_epoch_seconds,
            } => {
                record_sha256 == &super::digest(_record_bytes)
                    && *source_class == _record.source_class
                    && *observed_at_epoch_seconds == _record.observed_at_epoch_seconds
                    && *valid_until_epoch_seconds == _record.valid_until_epoch_seconds
                    && source_class.supports_current_capability_decision()
                    && !_record.has_externally_untrusted_stable_fact()
            }
        }
    }
}

impl ResearchSource {
    pub fn from_bound_record(
        snapshot: BoundInput,
        record_bytes: Vec<u8>,
    ) -> Result<Self, EvaluationError> {
        if record_bytes.is_empty()
            || record_bytes.len() > MAX_SOURCE_RECORD_BYTES
            || !snapshot.findings("research-source").is_empty()
            || snapshot.byte_length != record_bytes.len() as u64
            || snapshot.digest_sha256() != super::digest(&record_bytes)
        {
            return Err(invalid_source());
        }
        let record: ResearchSourceRecord = serde_json::from_slice(&record_bytes)
            .map_err(|_| EvaluationError::new("evaluation-research-source-record-invalid"))?;
        let value = Self {
            snapshot,
            record,
            record_bytes,
            authority_binding: ResearchSourceAuthorityBinding::UnverifiedExternal,
            mutation_revision: 0,
        };
        value.validate_shape()?;
        Ok(value)
    }

    /// Test-only seam for proving the behavior of a source whose classification
    /// and observation were adopted outside the caller-controlled record.  No
    /// corresponding production mint exists yet, so production conservatively
    /// rejects every source built by `from_bound_record`.
    #[cfg(test)]
    pub(crate) fn test_only_from_root_adopted_record(
        snapshot: BoundInput,
        record_bytes: Vec<u8>,
    ) -> Result<Self, EvaluationError> {
        let mut value = Self::from_bound_record(snapshot, record_bytes)?;
        if !value
            .record
            .source_class
            .supports_current_capability_decision()
            || value.record.has_externally_untrusted_stable_fact()
        {
            return Err(invalid_source());
        }
        value.authority_binding = ResearchSourceAuthorityBinding::TestRootAdopted {
            record_sha256: super::digest(&value.record_bytes),
            source_class: value.record.source_class,
            observed_at_epoch_seconds: value.record.observed_at_epoch_seconds,
            valid_until_epoch_seconds: value.record.valid_until_epoch_seconds,
        };
        Ok(value)
    }

    pub fn source_id(&self) -> &str {
        self.record.source_id()
    }

    pub fn class(&self) -> ResearchSourceClass {
        self.record.source_class()
    }

    pub fn snapshot(&self) -> &BoundInput {
        &self.snapshot
    }

    pub fn record(&self) -> &ResearchSourceRecord {
        &self.record
    }

    pub fn supports_proposal_ids(&self) -> &BTreeSet<String> {
        self.record.supports_proposal_ids()
    }

    fn validate_shape(&self) -> Result<(), EvaluationError> {
        if self.record_bytes.is_empty()
            || self.record_bytes.len() > MAX_SOURCE_RECORD_BYTES
            || !self.snapshot.findings("research-source").is_empty()
            || self.snapshot.byte_length != self.record_bytes.len() as u64
            || self.snapshot.digest_sha256() != super::digest(&self.record_bytes)
            || self.mutation_revision != 0
            || self.record.validate().is_err()
        {
            return Err(invalid_source());
        }
        let parsed: ResearchSourceRecord = serde_json::from_slice(&self.record_bytes)
            .map_err(|_| EvaluationError::new("evaluation-research-source-record-invalid"))?;
        if parsed != self.record || parsed.canonical_bytes() != self.record_bytes {
            return Err(EvaluationError::new(
                "evaluation-research-source-record-noncanonical-or-substituted",
            ));
        }
        Ok(())
    }

    fn has_adopted_authority_binding(&self) -> bool {
        self.authority_binding
            .validates(&self.record, &self.record_bytes)
    }

    #[cfg(test)]
    pub(crate) fn test_only_unchecked(
        snapshot: BoundInput,
        record: ResearchSourceRecord,
        record_bytes: Vec<u8>,
    ) -> Self {
        Self {
            snapshot,
            record,
            record_bytes,
            authority_binding: ResearchSourceAuthorityBinding::UnverifiedExternal,
            mutation_revision: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn substitute_snapshot_for_test(&mut self, snapshot: BoundInput) {
        self.snapshot = snapshot;
        self.mutation_revision = self.mutation_revision.saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn substitute_url_for_test(&mut self, url: impl Into<String>) {
        self.record.url = url.into();
        self.mutation_revision = self.mutation_revision.saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn substitute_record_bytes_for_test(&mut self, bytes: Vec<u8>) {
        self.record_bytes = bytes;
        self.mutation_revision = self.mutation_revision.saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn invalidate_classification_for_test(&mut self, control: &str) {
        match control {
            "fact-source-substitution" => {
                self.record.verified_source_facts[0].source_id = "source-substituted".to_owned();
            }
            "fact-advice-laundering" => {
                self.record.advisory_practices[0].practice_id =
                    self.record.verified_source_facts[0].fact_id.clone();
                self.record.advisory_practices[0].statement =
                    self.record.verified_source_facts[0].statement.clone();
            }
            "requirement-advice-laundering" => {
                self.record.advisory_practices[0].practice_id =
                    self.record.binding_product_requirements[0]
                        .requirement_id
                        .clone();
                self.record.advisory_practices[0].statement =
                    self.record.binding_product_requirements[0]
                        .statement
                        .clone();
            }
            "fact-hypothesis-laundering" => {
                self.record.experimental_hypotheses[0].hypothesis_id =
                    self.record.verified_source_facts[0].fact_id.clone();
                self.record.experimental_hypotheses[0].statement =
                    self.record.verified_source_facts[0].statement.clone();
            }
            "requirement-hypothesis-laundering" => {
                self.record.experimental_hypotheses[0].hypothesis_id =
                    self.record.binding_product_requirements[0]
                        .requirement_id
                        .clone();
                self.record.experimental_hypotheses[0].statement =
                    self.record.binding_product_requirements[0]
                        .statement
                        .clone();
            }
            _ => panic!("unknown research classification test control"),
        }
        self.rebind_test_bytes_without_revision();
    }

    #[cfg(test)]
    pub(crate) fn substitute_typed_field_for_test(&mut self, control: &str) {
        match control {
            "checked-date" => self.record.checked_date = "2026-07-12".to_owned(),
            "source-class" => {
                self.record.source_class = ResearchSourceClass::VendorDocumentation;
            }
            "limitation" => {
                self.record.limitations[0] = "Substituted limitation.".to_owned();
            }
            "fact" => {
                self.record.verified_source_facts[0].statement =
                    "Substituted verified fact.".to_owned();
            }
            "mapped-law" => {
                self.record.mapped_law_ids = BTreeSet::from(["HUL-OTHER-001".to_owned()]);
            }
            _ => panic!("unknown research field substitution test control"),
        }
        self.mutation_revision = self.mutation_revision.saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn substitute_class_for_test(&mut self, source_class: ResearchSourceClass) {
        self.record.source_class = source_class;
        self.rebind_test_bytes_without_revision();
    }

    #[cfg(test)]
    pub(crate) fn invalidate_freshness_for_test(&mut self, control: &str) {
        match control {
            "old-checked-date-current-observation" => {
                self.record.checked_date = "2000-01-01".to_owned();
            }
            "source-declared-indefinite-validity" => {
                self.record.valid_until_epoch_seconds = u64::MAX;
            }
            _ => panic!("unknown research freshness test control"),
        }
        self.rebind_test_bytes_without_revision();
    }

    #[cfg(test)]
    fn rebind_test_bytes_without_revision(&mut self) {
        self.record_bytes = self.record.canonical_bytes();
        self.snapshot = BoundInput::regular(
            self.snapshot.relative_path().to_owned(),
            super::digest(&self.record_bytes),
            self.record_bytes.len() as u64,
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ImpactAnalysis {
    summary: String,
    affected_requirement_ids: BTreeSet<String>,
    affected_product_surfaces: BTreeSet<String>,
    mapped_law_ids: BTreeSet<String>,
}

impl ImpactAnalysis {
    pub fn new(
        summary: impl Into<String>,
        affected_requirement_ids: BTreeSet<String>,
        affected_product_surfaces: BTreeSet<String>,
        mapped_law_ids: BTreeSet<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            summary: summary.into(),
            affected_requirement_ids,
            affected_product_surfaces,
            mapped_law_ids,
        };
        if !value.valid() {
            return Err(invalid_proposal());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_text(&self.summary, MAX_TEXT_BYTES)
            && valid_identifier_set(&self.affected_requirement_ids, MAX_CLASSIFICATION_ROWS)
            && valid_identifier_set(&self.affected_product_surfaces, MAX_CLASSIFICATION_ROWS)
            && valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MigrationAnalysis {
    summary: String,
    migration_steps: Vec<String>,
    rollback_steps: Vec<String>,
    mapped_law_ids: BTreeSet<String>,
}

impl MigrationAnalysis {
    pub fn new(
        summary: impl Into<String>,
        migration_steps: Vec<String>,
        rollback_steps: Vec<String>,
        mapped_law_ids: BTreeSet<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            summary: summary.into(),
            migration_steps,
            rollback_steps,
            mapped_law_ids,
        };
        if !value.valid() {
            return Err(invalid_proposal());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_text(&self.summary, MAX_TEXT_BYTES)
            && valid_text_list(&self.migration_steps)
            && valid_text_list(&self.rollback_steps)
            && valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProofAnalysis {
    summary: String,
    proof_obligations: BTreeSet<String>,
    negative_controls: Vec<String>,
    mapped_law_ids: BTreeSet<String>,
}

impl ProofAnalysis {
    pub fn new(
        summary: impl Into<String>,
        proof_obligations: BTreeSet<String>,
        negative_controls: Vec<String>,
        mapped_law_ids: BTreeSet<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            summary: summary.into(),
            proof_obligations,
            negative_controls,
            mapped_law_ids,
        };
        if !value.valid() {
            return Err(invalid_proposal());
        }
        Ok(value)
    }

    fn valid(&self) -> bool {
        valid_text(&self.summary, MAX_TEXT_BYTES)
            && valid_identifier_set(&self.proof_obligations, MAX_CLASSIFICATION_ROWS)
            && valid_text_list(&self.negative_controls)
            && valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AuthorityAnalysis {
    summary: String,
    required_decision_id: String,
    required_reviewer_role: String,
    supporting_source_ids: BTreeSet<String>,
    mapped_law_ids: BTreeSet<String>,
    authority_effect: NoAuthorityEffect,
}

impl AuthorityAnalysis {
    pub fn root_review_required(
        summary: impl Into<String>,
        supporting_source_ids: BTreeSet<String>,
        mapped_law_ids: BTreeSet<String>,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            summary: summary.into(),
            required_decision_id: ROOT_RESEARCH_DECISION_ID.to_owned(),
            required_reviewer_role: ROOT_RESEARCH_REVIEWER_ROLE.to_owned(),
            supporting_source_ids,
            mapped_law_ids,
            authority_effect: NoAuthorityEffect::None,
        };
        if !value.valid() {
            return Err(invalid_proposal());
        }
        Ok(value)
    }

    pub fn authority_effect(&self) -> &'static str {
        self.authority_effect.as_str()
    }

    fn valid(&self) -> bool {
        valid_text(&self.summary, MAX_TEXT_BYTES)
            && self.required_decision_id == ROOT_RESEARCH_DECISION_ID
            && self.required_reviewer_role == ROOT_RESEARCH_REVIEWER_ROLE
            && valid_identifier_set(&self.supporting_source_ids, MAX_RESEARCH_SOURCES)
            && valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
            && self.authority_effect == NoAuthorityEffect::None
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProposalAnalyses {
    impact: ImpactAnalysis,
    migration: MigrationAnalysis,
    proof: ProofAnalysis,
    authority: AuthorityAnalysis,
}

impl ProposalAnalyses {
    pub fn new(
        impact: ImpactAnalysis,
        migration: MigrationAnalysis,
        proof: ProofAnalysis,
        authority: AuthorityAnalysis,
    ) -> Self {
        Self {
            impact,
            migration,
            proof,
            authority,
        }
    }

    fn valid_for(
        &self,
        supporting_source_ids: &BTreeSet<String>,
        mapped_law_ids: &BTreeSet<String>,
    ) -> bool {
        self.impact.valid()
            && self.migration.valid()
            && self.proof.valid()
            && self.authority.valid()
            && self.impact.mapped_law_ids == *mapped_law_ids
            && self.migration.mapped_law_ids == *mapped_law_ids
            && self.proof.mapped_law_ids == *mapped_law_ids
            && self.authority.mapped_law_ids == *mapped_law_ids
            && self.authority.supporting_source_ids == *supporting_source_ids
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LawChangeProposal {
    proposal_id: String,
    title: String,
    rationale: String,
    supporting_source_ids: BTreeSet<String>,
    mapped_law_ids: BTreeSet<String>,
    analyses: ProposalAnalyses,
    authority_effect: NoAuthorityEffect,
}

impl LawChangeProposal {
    pub fn non_authoritative(
        proposal_id: impl Into<String>,
        title: impl Into<String>,
        rationale: impl Into<String>,
        supporting_source_ids: BTreeSet<String>,
        mapped_law_ids: BTreeSet<String>,
        analyses: ProposalAnalyses,
    ) -> Result<Self, EvaluationError> {
        let value = Self {
            proposal_id: proposal_id.into(),
            title: title.into(),
            rationale: rationale.into(),
            supporting_source_ids,
            mapped_law_ids,
            analyses,
            authority_effect: NoAuthorityEffect::None,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn proposal_id(&self) -> &str {
        &self.proposal_id
    }

    pub fn supporting_source_ids(&self) -> &BTreeSet<String> {
        &self.supporting_source_ids
    }

    pub fn mapped_law_ids(&self) -> &BTreeSet<String> {
        &self.mapped_law_ids
    }

    pub fn authority_effect(&self) -> &'static str {
        self.authority_effect.as_str()
    }

    fn validate(&self) -> Result<(), EvaluationError> {
        if !valid_identifier(&self.proposal_id)
            || !valid_text(&self.title, 160)
            || !valid_text(&self.rationale, 2_048)
            || !valid_identifier_set(&self.supporting_source_ids, MAX_RESEARCH_SOURCES)
            || !valid_identifier_set(&self.mapped_law_ids, MAX_CLASSIFICATION_ROWS)
            || !self
                .analyses
                .valid_for(&self.supporting_source_ids, &self.mapped_law_ids)
            || self.authority_effect != NoAuthorityEffect::None
        {
            return Err(invalid_proposal());
        }
        Ok(())
    }

    fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.authority_effect = NoAuthorityEffect::None;
        normalized.analyses.authority.authority_effect = NoAuthorityEffect::None;
        normalized
    }

    #[cfg(test)]
    pub(crate) fn test_only_clear_analysis(&mut self, analysis: &str) {
        match analysis {
            "impact" => self.analyses.impact.summary.clear(),
            "migration" => self.analyses.migration.summary.clear(),
            "proof" => self.analyses.proof.summary.clear(),
            "authority" => self.analyses.authority.summary.clear(),
            _ => panic!("unknown proposal analysis test seam"),
        }
    }

    #[cfg(test)]
    pub(crate) fn test_only_substitute_supporting_sources(
        &mut self,
        supporting_source_ids: BTreeSet<String>,
    ) {
        self.supporting_source_ids = supporting_source_ids;
    }

    #[cfg(test)]
    pub(crate) fn test_only_substitute_mapped_laws(&mut self, mapped_law_ids: BTreeSet<String>) {
        self.mapped_law_ids = mapped_law_ids;
    }

    #[cfg(test)]
    pub(crate) fn test_only_substitute_authority(&mut self, control: &str) {
        match control {
            "decision" => {
                self.analyses.authority.required_decision_id = "OD-WORKER-ADOPTION".to_owned();
            }
            "reviewer" => {
                self.analyses.authority.required_reviewer_role = "worker".to_owned();
            }
            _ => panic!("unknown proposal authority test control"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResearchFinding {
    code: String,
    source_id: Option<String>,
    proposal_id: Option<String>,
    authority_effect: NoAuthorityEffect,
}

impl ResearchFinding {
    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn source_id(&self) -> Option<&str> {
        self.source_id.as_deref()
    }

    pub fn proposal_id(&self) -> Option<&str> {
        self.proposal_id.as_deref()
    }

    pub fn authority_effect(&self) -> &'static str {
        self.authority_effect.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ResearchAudit {
    eligible_proposals: Vec<LawChangeProposal>,
    findings: Vec<ResearchFinding>,
    authority_effect: NoAuthorityEffect,
}

impl ResearchAudit {
    pub fn audit(sources: &[ResearchSource], proposals: &[LawChangeProposal]) -> Self {
        let current_epoch_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_secs());
        Self::audit_with_current_time(sources, proposals, current_epoch_seconds)
    }

    #[cfg(test)]
    pub(crate) fn test_only_audit_at(
        sources: &[ResearchSource],
        proposals: &[LawChangeProposal],
        current_epoch_seconds: u64,
    ) -> Self {
        Self::audit_with_current_time(sources, proposals, Some(current_epoch_seconds))
    }

    #[cfg(test)]
    pub(crate) fn test_only_audit_with_clock_failure(
        sources: &[ResearchSource],
        proposals: &[LawChangeProposal],
    ) -> Self {
        Self::audit_with_current_time(sources, proposals, None)
    }

    fn audit_with_current_time(
        sources: &[ResearchSource],
        proposals: &[LawChangeProposal],
        current_epoch_seconds: Option<u64>,
    ) -> Self {
        let mut findings = Vec::new();
        let Some(current_epoch_seconds) = current_epoch_seconds else {
            findings.push(finding("research-clock-unavailable", None, None));
            return Self::with(findings, Vec::new());
        };
        if current_epoch_seconds == 0 || current_epoch_seconds == u64::MAX {
            findings.push(finding("research-clock-invalid", None, None));
            return Self::with(findings, Vec::new());
        }
        if sources.is_empty() || sources.len() > MAX_RESEARCH_SOURCES {
            findings.push(finding("research-source-count-out-of-bounds", None, None));
        }
        if proposals.is_empty() || proposals.len() > MAX_PROPOSALS {
            findings.push(finding("research-proposal-count-out-of-bounds", None, None));
        }
        if !findings.is_empty() {
            return Self::with(findings, Vec::new());
        }

        let mut source_id_counts = BTreeMap::new();
        let mut snapshot_digest_counts = BTreeMap::new();
        for source in sources {
            *source_id_counts
                .entry(source.source_id())
                .or_insert(0_usize) += 1;
            *snapshot_digest_counts
                .entry(source.snapshot.digest_sha256())
                .or_insert(0_usize) += 1;
        }
        let mut valid_sources = BTreeMap::new();
        for source in sources {
            let source_reference = valid_reference(source.source_id());
            let shape_valid = source.validate_shape().is_ok();
            if !shape_valid {
                findings.push(finding("research-source-invalid", source_reference, None));
            }
            let authority_bound = source.has_adopted_authority_binding();
            if !authority_bound {
                findings.push(finding(
                    "research-source-authority-binding-required",
                    source_reference,
                    None,
                ));
            }
            let current = source.record.is_current_at(current_epoch_seconds);
            if !current {
                findings.push(finding("research-source-stale", source_reference, None));
            }
            let temporal_scope_supported = !source.record.has_externally_untrusted_stable_fact();
            if !temporal_scope_supported {
                findings.push(finding(
                    "research-source-stable-fact-authority-required",
                    source_reference,
                    None,
                ));
            }
            let primary_supported = !source.record.requires_current_primary_source()
                || source.class().supports_current_capability_decision();
            if !primary_supported {
                findings.push(finding(
                    "research-source-current-primary-required",
                    source_reference,
                    None,
                ));
            }
            let duplicate = source_id_counts
                .get(source.source_id())
                .is_some_and(|count| *count > 1)
                || snapshot_digest_counts
                    .get(source.snapshot.digest_sha256())
                    .is_some_and(|count| *count > 1);
            if duplicate {
                findings.push(finding("research-source-duplicate", source_reference, None));
            }
            if shape_valid
                && authority_bound
                && current
                && temporal_scope_supported
                && primary_supported
                && !duplicate
            {
                valid_sources.insert(source.source_id(), source);
            }
        }

        let mut proposal_id_counts = BTreeMap::new();
        for proposal in proposals {
            *proposal_id_counts
                .entry(proposal.proposal_id.as_str())
                .or_insert(0_usize) += 1;
        }
        let mut eligible_proposals = Vec::new();
        for proposal in proposals {
            let proposal_reference = valid_reference(&proposal.proposal_id);
            let shape_valid = proposal.validate().is_ok();
            if !shape_valid {
                findings.push(finding(
                    "research-proposal-invalid",
                    None,
                    proposal_reference,
                ));
            }
            let duplicate = proposal_id_counts
                .get(proposal.proposal_id.as_str())
                .is_some_and(|count| *count > 1);
            if duplicate {
                findings.push(finding(
                    "research-proposal-duplicate",
                    None,
                    proposal_reference,
                ));
            }

            let mut supported_laws = BTreeSet::new();
            let support_valid = proposal.supporting_source_ids.iter().all(|source_id| {
                valid_sources.get(source_id.as_str()).is_some_and(|source| {
                    if source
                        .supports_proposal_ids()
                        .contains(&proposal.proposal_id)
                    {
                        supported_laws.extend(source.record.mapped_law_ids().iter().cloned());
                        true
                    } else {
                        false
                    }
                })
            });
            let mapped_laws_valid = proposal.mapped_law_ids.is_subset(&supported_laws);
            if !support_valid || !mapped_laws_valid {
                findings.push(finding(
                    "research-proposal-support-invalid",
                    None,
                    proposal_reference,
                ));
            }
            if shape_valid && !duplicate && support_valid && mapped_laws_valid {
                eligible_proposals.push(proposal.normalized());
            }
        }

        Self::with(findings, eligible_proposals)
    }

    pub fn eligible_proposals(&self) -> &[LawChangeProposal] {
        &self.eligible_proposals
    }

    pub fn findings(&self) -> &[ResearchFinding] {
        &self.findings
    }

    pub fn authority_effect(&self) -> &'static str {
        self.authority_effect.as_str()
    }

    fn with(
        mut findings: Vec<ResearchFinding>,
        eligible_proposals: Vec<LawChangeProposal>,
    ) -> Self {
        findings.sort_by(|left, right| {
            (&left.code, &left.source_id, &left.proposal_id).cmp(&(
                &right.code,
                &right.source_id,
                &right.proposal_id,
            ))
        });
        Self {
            eligible_proposals,
            findings,
            authority_effect: NoAuthorityEffect::None,
        }
    }
}

fn invalid_source() -> EvaluationError {
    EvaluationError::new("evaluation-research-source-invalid")
}

fn invalid_proposal() -> EvaluationError {
    EvaluationError::new("evaluation-research-proposal-invalid")
}

fn valid_identifier(value: &str) -> bool {
    super::valid_identifier(value)
}

fn valid_identifier_set(values: &BTreeSet<String>, max: usize) -> bool {
    !values.is_empty() && values.len() <= max && values.iter().all(|value| valid_identifier(value))
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

fn valid_text_list(values: &[String]) -> bool {
    !values.is_empty()
        && values.len() <= MAX_CLASSIFICATION_ROWS
        && values.iter().all(|value| valid_text(value, MAX_TEXT_BYTES))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_nonempty_rows<T>(values: &[T]) -> bool {
    !values.is_empty() && values.len() <= MAX_CLASSIFICATION_ROWS
}

fn valid_https_url(value: &str) -> bool {
    if value.len() > MAX_URL_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || value.contains('#')
    {
        return false;
    }
    let Some(authority_and_path) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = authority_and_path.split('/').next().unwrap_or_default();
    !authority.is_empty() && authority.contains('.') && !authority.contains('@')
}

fn valid_https_locator(value: &str) -> bool {
    let base = value.split('#').next().unwrap_or_default();
    valid_https_url(base)
        && value
            .strip_prefix(base)
            .is_some_and(|suffix| suffix.is_empty() || valid_fragment(suffix))
}

fn valid_fragment(value: &str) -> bool {
    value
        .strip_prefix('#')
        .is_some_and(|fragment| !fragment.is_empty() && valid_identifier(fragment))
}

fn valid_source_locator(locator: &str, source_url: &str) -> bool {
    locator == source_url || locator.strip_prefix(source_url).is_some_and(valid_fragment)
}

fn freshness_window_seconds(_facts: &[VerifiedSourceFact]) -> u64 {
    MUTABLE_CLAIM_FRESHNESS_SECONDS
}

fn parsed_checked_date(value: &str) -> Option<(u16, u8, u8)> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return None;
    }
    let year = value[0..4].parse::<u16>().unwrap_or(0);
    let month = value[5..7].parse::<u8>().unwrap_or(0);
    let day = value[8..10].parse::<u8>().unwrap_or(0);
    if year < 2000 || !(1..=12).contains(&month) {
        return None;
    }
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month)
        .contains(&day)
        .then_some((year, month, day))
}

fn checked_date_matches_epoch(value: &str, epoch_seconds: u64) -> bool {
    let Some((year, month, day)) = parsed_checked_date(value) else {
        return false;
    };
    let days_before_year = (1970..year)
        .map(|candidate| {
            if candidate.is_multiple_of(4)
                && (!candidate.is_multiple_of(100) || candidate.is_multiple_of(400))
            {
                366_u64
            } else {
                365_u64
            }
        })
        .sum::<u64>();
    let days_before_month = (1..month)
        .map(|candidate| match candidate {
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29_u64
            }
            2 => 28_u64,
            4 | 6 | 9 | 11 => 30_u64,
            _ => 31_u64,
        })
        .sum::<u64>();
    epoch_seconds / SECONDS_PER_DAY == days_before_year + days_before_month + u64::from(day - 1)
}

fn valid_sourced_row(
    id: &str,
    statement: &str,
    source_id: &str,
    source_locator: &str,
    mapped_law_ids: &BTreeSet<String>,
) -> bool {
    valid_identifier(id)
        && valid_text(statement, MAX_TEXT_BYTES)
        && valid_identifier(source_id)
        && valid_https_locator(source_locator)
        && valid_identifier_set(mapped_law_ids, MAX_CLASSIFICATION_ROWS)
}

fn valid_record_sourced_row<'a>(
    id: &'a str,
    statement: &'a str,
    source_id: &str,
    source_locator: &str,
    mapped_law_ids: &BTreeSet<String>,
    record: &ResearchSourceRecord,
    row_ids: &mut BTreeSet<&'a str>,
    statements: &mut BTreeSet<&'a str>,
) -> bool {
    source_id == record.source_id
        && valid_source_locator(source_locator, &record.url)
        && law_subset(mapped_law_ids, &record.mapped_law_ids)
        && row_ids.insert(id)
        && statements.insert(statement)
}

fn law_subset(values: &BTreeSet<String>, supported: &BTreeSet<String>) -> bool {
    !values.is_empty() && values.is_subset(supported)
}

fn valid_reference(value: &str) -> Option<&str> {
    valid_identifier(value).then_some(value)
}

fn finding(code: &str, source_id: Option<&str>, proposal_id: Option<&str>) -> ResearchFinding {
    ResearchFinding {
        code: code.to_owned(),
        source_id: source_id.map(str::to_owned),
        proposal_id: proposal_id.map(str::to_owned),
        authority_effect: NoAuthorityEffect::None,
    }
}
