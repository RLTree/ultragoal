impl ResearchSource {
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
