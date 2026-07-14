pub struct EvaluationTaskDefinition {
    pub task_id: String,
    pub requirement_id: String,
    pub behavior_id: String,
    pub fixture_id: String,
    pub dataset: BoundInput,
    pub scorer_id: String,
    pub scorer_digest_sha256: String,
    pub perturbation_controls: BTreeSet<PerturbationControl>,
    pub representative: bool,
}

pub struct EvaluationDatasetProvenance {
    pub dataset_provenance_sha256: String,
    pub known_training_corpus_sha256s: BTreeSet<String>,
}

impl EvaluationTask {
    pub fn new(definition: EvaluationTaskDefinition) -> Self {
        let provenance =
            digest(format!("dataset-provenance|{}", definition.dataset.digest_sha256()).as_bytes());
        Self::new_with_provenance(
            definition,
            EvaluationDatasetProvenance {
                dataset_provenance_sha256: provenance,
                known_training_corpus_sha256s: BTreeSet::new(),
            },
        )
    }

    pub fn new_with_provenance(
        definition: EvaluationTaskDefinition,
        provenance: EvaluationDatasetProvenance,
    ) -> Self {
        let EvaluationTaskDefinition {
            task_id,
            requirement_id,
            behavior_id,
            fixture_id,
            dataset,
            scorer_id,
            scorer_digest_sha256,
            perturbation_controls,
            representative,
        } = definition;
        let EvaluationDatasetProvenance {
            dataset_provenance_sha256,
            known_training_corpus_sha256s,
        } = provenance;
        let data_controls =
            EvaluationDataControls::defaults(&task_id, &behavior_id, dataset.digest_sha256());
        Self {
            task_id,
            requirement_id,
            behavior_id,
            fixture_id,
            dataset,
            dataset_provenance_sha256,
            known_training_corpus_sha256s,
            scorer_id,
            scorer_digest_sha256,
            perturbation_controls,
            representative,
            data_controls,
        }
    }

    pub fn with_data_controls(mut self, data_controls: EvaluationDataControls) -> Self {
        self.data_controls = data_controls;
        self
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn fixture_id(&self) -> &str {
        &self.fixture_id
    }

    fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        for (field, value) in [
            ("task", self.task_id.as_str()),
            ("requirement", self.requirement_id.as_str()),
            ("behavior", self.behavior_id.as_str()),
            ("fixture", self.fixture_id.as_str()),
            ("scorer", self.scorer_id.as_str()),
        ] {
            if !valid_identifier(value) {
                findings.push(format!("evaluation-{field}-id-invalid"));
            }
        }
        findings.extend(self.dataset.findings("evaluation-dataset"));
        findings.extend(self.data_controls.findings());
        if !valid_sha256(&self.dataset_provenance_sha256)
            || self
                .known_training_corpus_sha256s
                .iter()
                .any(|digest| !valid_sha256(digest))
        {
            findings.push("evaluation-dataset-provenance-invalid".to_owned());
        }
        if self
            .known_training_corpus_sha256s
            .contains(self.dataset.digest_sha256())
        {
            findings.push("evaluation-dataset-contamination-detected".to_owned());
        }
        if !valid_sha256(&self.scorer_digest_sha256) {
            findings.push("evaluation-scorer-digest-invalid".to_owned());
        }
        for required in PerturbationControl::REQUIRED {
            if !self.perturbation_controls.contains(&required) {
                findings.push(format!(
                    "evaluation-perturbation-control-missing:{}",
                    required.label()
                ));
            }
        }
        findings
    }

    fn digest_fragment(&self) -> String {
        let controls = self
            .perturbation_controls
            .iter()
            .copied()
            .map(PerturbationControl::label)
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.task_id,
            self.requirement_id,
            self.behavior_id,
            self.fixture_id,
            self.dataset.digest_fragment(),
            self.dataset_provenance_sha256,
            self.known_training_corpus_sha256s
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
            self.scorer_id,
            self.scorer_digest_sha256,
            controls,
            self.representative,
            self.data_controls.digest_fragment(),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EvaluationSpec {
    live_context_id: String,
    candidate_id: String,
    spec_id: String,
    tasks: Vec<EvaluationTask>,
    task_set_sha256: String,
    spec_sha256: String,
    revision: u64,
}
