impl EvaluationSpec {
    pub fn new(
        live_context_id: impl Into<String>,
        candidate_id: impl Into<String>,
        spec_id: impl Into<String>,
        mut tasks: Vec<EvaluationTask>,
    ) -> Result<Self, EvaluationError> {
        if tasks.is_empty() || tasks.len() > MAX_TASKS {
            return Err(EvaluationError::new("evaluation-task-count-out-of-bounds"));
        }
        tasks.sort_by(|left, right| left.task_id.cmp(&right.task_id));
        let live_context_id = live_context_id.into();
        let candidate_id = candidate_id.into();
        let spec_id = spec_id.into();
        let task_set_sha256 = digest(
            tasks
                .iter()
                .map(EvaluationTask::digest_fragment)
                .collect::<Vec<_>>()
                .join("\n")
                .as_bytes(),
        );
        let spec_sha256 = digest(
            format!("{live_context_id}|{candidate_id}|{spec_id}|{task_set_sha256}").as_bytes(),
        );
        Ok(Self {
            live_context_id,
            candidate_id,
            spec_id,
            tasks,
            task_set_sha256,
            spec_sha256,
            revision: 0,
        })
    }

    pub fn audit(&self, current_context_id: &str, current_candidate_id: &str) -> TaskAudit {
        let mut findings = Vec::new();
        if self.live_context_id != current_context_id {
            findings.push("evaluation-context-stale".to_owned());
        }
        if self.candidate_id != current_candidate_id {
            findings.push("evaluation-candidate-stale".to_owned());
        }
        if !valid_sha256(&self.live_context_id) || !valid_sha256(&self.candidate_id) {
            findings.push("evaluation-binding-invalid".to_owned());
        }
        if !valid_identifier(&self.spec_id) {
            findings.push("evaluation-spec-id-invalid".to_owned());
        }
        let mut task_ids = BTreeSet::new();
        let mut fixture_ids = BTreeSet::new();
        let mut dataset_digests = BTreeMap::<&str, &str>::new();
        let mut unique_dataset_digests = BTreeSet::new();
        let mut semantic_fingerprints = BTreeSet::new();
        let mut near_duplicate_groups = BTreeSet::new();
        let mut representative = 0usize;
        for task in &self.tasks {
            if !task_ids.insert(task.task_id.as_str()) {
                findings.push("evaluation-duplicate-task".to_owned());
            }
            if !fixture_ids.insert(task.fixture_id.as_str()) {
                findings.push("evaluation-duplicate-fixture".to_owned());
            }
            if task.representative {
                representative += 1;
            }
            if let Some(existing) =
                dataset_digests.insert(task.dataset.relative_path(), task.dataset.digest_sha256())
                && existing != task.dataset.digest_sha256()
            {
                findings.push("evaluation-dataset-path-conflict".to_owned());
            }
            if !unique_dataset_digests.insert(task.dataset.digest_sha256()) {
                findings.push("evaluation-duplicate-dataset".to_owned());
            }
            if !semantic_fingerprints
                .insert(task.data_controls.semantic_fingerprint_sha256.as_str())
            {
                findings.push("evaluation-duplicate-example".to_owned());
            }
            if !near_duplicate_groups
                .insert(task.data_controls.near_duplicate_group_sha256.as_str())
            {
                findings.push("evaluation-near-duplicate-example".to_owned());
            }
            findings.extend(task.findings());
        }
        if representative == 0 {
            findings.push("evaluation-representative-task-missing".to_owned());
        }
        if self.task_set_sha256
            != digest(
                self.tasks
                    .iter()
                    .map(EvaluationTask::digest_fragment)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .as_bytes(),
            )
            || self.spec_sha256
                != digest(
                    format!(
                        "{}|{}|{}|{}",
                        self.live_context_id, self.candidate_id, self.spec_id, self.task_set_sha256
                    )
                    .as_bytes(),
                )
        {
            findings.push("evaluation-spec-mutated".to_owned());
        }
        findings.sort();
        findings.dedup();
        TaskAudit {
            live_context_id: current_context_id.to_owned(),
            candidate_id: current_candidate_id.to_owned(),
            spec_sha256: self.spec_sha256.clone(),
            spec_revision: self.revision,
            eligible: findings.is_empty(),
            findings,
        }
    }

    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    pub fn live_context_id(&self) -> &str {
        &self.live_context_id
    }

    pub fn spec_id(&self) -> &str {
        &self.spec_id
    }

    pub fn spec_sha256(&self) -> &str {
        &self.spec_sha256
    }

    pub fn task_set_sha256(&self) -> &str {
        &self.task_set_sha256
    }

    #[cfg(test)]
    pub(crate) fn corrupt_candidate_for_test(&mut self, candidate_id: impl Into<String>) {
        self.candidate_id = candidate_id.into();
        self.revision = self.revision.saturating_add(1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TaskAudit {
    live_context_id: String,
    candidate_id: String,
    spec_sha256: String,
    spec_revision: u64,
    eligible: bool,
    findings: Vec<String>,
}

impl TaskAudit {
    pub fn eligible(&self) -> bool {
        self.eligible
    }

    pub fn findings(&self) -> &[String] {
        &self.findings
    }

    #[cfg(test)]
    fn revalidate(&self, spec: &EvaluationSpec, context: &str, candidate: &str) -> bool {
        self.eligible
            && self.live_context_id == context
            && self.candidate_id == candidate
            && self.spec_sha256 == spec.spec_sha256
            && self.spec_revision == spec.revision
            && spec.audit(context, candidate).eligible
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorOutcome {
    Passed,
    Failed,
    Quarantined,
}
