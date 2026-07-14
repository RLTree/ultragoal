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
}
