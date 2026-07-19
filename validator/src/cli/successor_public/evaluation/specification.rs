use super::*;
use crate::evaluation::{
    BoundInput, EvaluationDataControls, EvaluationDataControlsDefinition,
    EvaluationDatasetProvenance, EvaluationSpec, EvaluationTask, EvaluationTaskDefinition,
    PerturbationControl,
};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

const MAX_SPEC_BYTES: u64 = 4 * 1024 * 1024;
const SCHEMA_VERSION: &str = "EvaluationAuditSpec-v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditSpecificationInput {
    schema_version: String,
    spec_id: String,
    tasks: Vec<AuditTaskInput>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditTaskInput {
    task_id: String,
    requirement_id: String,
    behavior_id: String,
    fixture_id: String,
    dataset: DatasetBindingInput,
    dataset_provenance_sha256: String,
    known_training_corpus_sha256s: BTreeSet<String>,
    scorer_id: String,
    scorer_digest_sha256: String,
    perturbation_controls: BTreeSet<PerturbationControl>,
    representative: bool,
    data_controls: DataControlInput,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetBindingInput {
    relative_path: String,
    digest_sha256: String,
    byte_length: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DataControlInput {
    objective: String,
    success_criterion: String,
    failure_criterion: String,
    split_id: String,
    training_split_ids: BTreeSet<String>,
    semantic_fingerprint_sha256: String,
    near_duplicate_group_sha256: String,
    known_training_fingerprint_sha256s: BTreeSet<String>,
    declared_label: String,
    verified_label: String,
    sampled_population: String,
    target_population: String,
}

pub(super) fn load(
    context: &LiveContext,
    spec_path: &str,
    candidate_id: &str,
) -> Result<EvaluationSpec, ()> {
    let reads = context.begin_read_session().map_err(|_| ())?;
    let bytes = reads
        .read_bounded(Path::new(spec_path), MAX_SPEC_BYTES)
        .map_err(|_| ())?;
    let input = serde_json::from_slice::<AuditSpecificationInput>(&bytes).map_err(|_| ())?;
    let spec = input.into_spec(context.context_id(), candidate_id)?;
    reads.revalidate().map_err(|_| ())?;
    Ok(spec)
}

impl AuditSpecificationInput {
    fn into_spec(self, context_id: &str, candidate_id: &str) -> Result<EvaluationSpec, ()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(());
        }
        let tasks = self
            .tasks
            .into_iter()
            .map(AuditTaskInput::into_task)
            .collect();
        EvaluationSpec::new(context_id, candidate_id, self.spec_id, tasks).map_err(|_| ())
    }
}

impl AuditTaskInput {
    fn into_task(self) -> EvaluationTask {
        let data_controls = EvaluationDataControls::new(EvaluationDataControlsDefinition {
            objective: self.data_controls.objective,
            success_criterion: self.data_controls.success_criterion,
            failure_criterion: self.data_controls.failure_criterion,
            split_id: self.data_controls.split_id,
            training_split_ids: self.data_controls.training_split_ids,
            semantic_fingerprint_sha256: self.data_controls.semantic_fingerprint_sha256,
            near_duplicate_group_sha256: self.data_controls.near_duplicate_group_sha256,
            known_training_fingerprint_sha256s: self
                .data_controls
                .known_training_fingerprint_sha256s,
            declared_label: self.data_controls.declared_label,
            verified_label: self.data_controls.verified_label,
            sampled_population: self.data_controls.sampled_population,
            target_population: self.data_controls.target_population,
        });
        let task = EvaluationTask::new_with_provenance(
            EvaluationTaskDefinition {
                task_id: self.task_id,
                requirement_id: self.requirement_id,
                behavior_id: self.behavior_id,
                fixture_id: self.fixture_id,
                dataset: BoundInput::regular(
                    self.dataset.relative_path,
                    self.dataset.digest_sha256,
                    self.dataset.byte_length,
                ),
                scorer_id: self.scorer_id,
                scorer_digest_sha256: self.scorer_digest_sha256,
                perturbation_controls: self.perturbation_controls,
                representative: self.representative,
            },
            EvaluationDatasetProvenance {
                dataset_provenance_sha256: self.dataset_provenance_sha256,
                known_training_corpus_sha256s: self.known_training_corpus_sha256s,
            },
        );
        task.with_data_controls(data_controls)
    }
}
