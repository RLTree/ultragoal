pub struct EvaluationDataControlsDefinition {
    pub objective: String,
    pub success_criterion: String,
    pub failure_criterion: String,
    pub split_id: String,
    pub training_split_ids: BTreeSet<String>,
    pub semantic_fingerprint_sha256: String,
    pub near_duplicate_group_sha256: String,
    pub known_training_fingerprint_sha256s: BTreeSet<String>,
    pub declared_label: String,
    pub verified_label: String,
    pub sampled_population: String,
    pub target_population: String,
}

impl EvaluationDataControls {
    pub fn new(definition: EvaluationDataControlsDefinition) -> Self {
        let EvaluationDataControlsDefinition {
            objective,
            success_criterion,
            failure_criterion,
            split_id,
            training_split_ids,
            semantic_fingerprint_sha256,
            near_duplicate_group_sha256,
            known_training_fingerprint_sha256s,
            declared_label,
            verified_label,
            sampled_population,
            target_population,
        } = definition;
        Self {
            objective,
            success_criterion,
            failure_criterion,
            split_id,
            training_split_ids,
            semantic_fingerprint_sha256,
            near_duplicate_group_sha256,
            known_training_fingerprint_sha256s,
            declared_label,
            verified_label,
            sampled_population,
            target_population,
        }
    }

    fn defaults(task_id: &str, behavior_id: &str, dataset_digest: &str) -> Self {
        Self::new(EvaluationDataControlsDefinition {
            objective: format!("observe-{behavior_id}"),
            success_criterion: format!("{behavior_id}-passes"),
            failure_criterion: format!("{behavior_id}-fails"),
            split_id: format!("eval-{task_id}"),
            training_split_ids: BTreeSet::new(),
            semantic_fingerprint_sha256: digest(
                format!("semantic|{task_id}|{dataset_digest}").as_bytes(),
            ),
            near_duplicate_group_sha256: digest(
                format!("near-duplicate|{task_id}|{dataset_digest}").as_bytes(),
            ),
            known_training_fingerprint_sha256s: BTreeSet::new(),
            declared_label: behavior_id.to_owned(),
            verified_label: behavior_id.to_owned(),
            sampled_population: "representative-population".to_owned(),
            target_population: "representative-population".to_owned(),
        })
    }

    fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if !valid_identifier(&self.objective)
            || !valid_identifier(&self.success_criterion)
            || !valid_identifier(&self.failure_criterion)
            || self.success_criterion == self.failure_criterion
        {
            findings.push("evaluation-task-ambiguous".to_owned());
        }
        if !valid_identifier(&self.split_id)
            || self
                .training_split_ids
                .iter()
                .any(|split| !valid_identifier(split))
            || self.training_split_ids.contains(&self.split_id)
        {
            findings.push("evaluation-split-leakage-detected".to_owned());
        }
        if !valid_sha256(&self.semantic_fingerprint_sha256)
            || !valid_sha256(&self.near_duplicate_group_sha256)
            || self
                .known_training_fingerprint_sha256s
                .iter()
                .any(|value| !valid_sha256(value))
        {
            findings.push("evaluation-data-fingerprint-invalid".to_owned());
        }
        if self
            .known_training_fingerprint_sha256s
            .contains(&self.semantic_fingerprint_sha256)
            || self
                .known_training_fingerprint_sha256s
                .contains(&self.near_duplicate_group_sha256)
        {
            findings.push("evaluation-near-duplicate-leakage-detected".to_owned());
        }
        if !valid_identifier(&self.declared_label)
            || !valid_identifier(&self.verified_label)
            || self.declared_label != self.verified_label
        {
            findings.push("evaluation-label-mismatch".to_owned());
        }
        if !valid_identifier(&self.sampled_population)
            || !valid_identifier(&self.target_population)
            || self.sampled_population != self.target_population
        {
            findings.push("evaluation-unrepresentative-data".to_owned());
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.objective,
            self.success_criterion,
            self.failure_criterion,
            self.split_id,
            self.training_split_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
            self.semantic_fingerprint_sha256,
            self.near_duplicate_group_sha256,
            self.known_training_fingerprint_sha256s
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
            self.declared_label,
            self.verified_label,
            self.sampled_population,
            self.target_population,
        )
    }
}
