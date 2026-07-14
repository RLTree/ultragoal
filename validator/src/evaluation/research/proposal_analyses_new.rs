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
