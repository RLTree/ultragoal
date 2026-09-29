use super::super::{EvaluationError, EvaluationSpec, digest};
use super::admission_authority::EvaluationAdmissionRequest;
use super::descriptor::{ProtectedProductionInput, open_production_root};
use serde::Deserialize;

#[derive(Deserialize)]
struct RootScorerEvidence {
    task_authority_id: String,
    task_author_principal_id: String,
    task_author_session_id: String,
}

#[derive(Deserialize)]
struct RootGraderEvidence {
    root_authority_id: String,
    independent_grader_principal_id: String,
    independent_grader_session_id: String,
}

pub(super) fn derive_root_admission(
    spec: &EvaluationSpec,
    root: &std::path::Path,
) -> Result<EvaluationAdmissionRequest, EvaluationError> {
    let root_file = open_production_root(root)?;
    let mut expected_task: Option<(String, String, String)> = None;
    let mut expected_grader: Option<(String, String, String)> = None;
    for task in &spec.tasks {
        let scorer = ProtectedProductionInput::capture(
            &root_file,
            &format!("scorers/{}", task.scorer_id),
            &task.scorer_digest_sha256,
            None,
        )?;
        let scorer: RootScorerEvidence = serde_json::from_slice(&scorer.read_authenticated()?)
            .map_err(|_| EvaluationError::new("evaluation-admission-invalid"))?;
        let grader = ProtectedProductionInput::capture_observed(
            &root_file,
            &format!("graders/{}.json", task.task_id),
        )?;
        let grader: RootGraderEvidence = serde_json::from_slice(&grader.read_authenticated()?)
            .map_err(|_| EvaluationError::new("evaluation-admission-invalid"))?;
        let task_row = (
            scorer.task_authority_id,
            scorer.task_author_principal_id,
            scorer.task_author_session_id,
        );
        let grader_row = (
            grader.root_authority_id,
            grader.independent_grader_principal_id,
            grader.independent_grader_session_id,
        );
        if expected_task.as_ref().is_some_and(|row| row != &task_row)
            || expected_grader
                .as_ref()
                .is_some_and(|row| row != &grader_row)
        {
            return Err(EvaluationError::new("evaluation-admission-invalid"));
        }
        expected_task = Some(task_row);
        expected_grader = Some(grader_row);
    }
    let (task_authority_id, task_principal_id, task_session_id) =
        expected_task.ok_or_else(|| EvaluationError::new("evaluation-admission-invalid"))?;
    let (grader_authority_id, grader_principal_id, grader_session_id) =
        expected_grader.ok_or_else(|| EvaluationError::new("evaluation-admission-invalid"))?;
    let provenance_seed = format!(
        "{}|{}|{}",
        spec.live_context_id(),
        spec.candidate_id(),
        spec.spec_sha256()
    );
    let provenance_digest = digest(provenance_seed.as_bytes());
    Ok(EvaluationAdmissionRequest::from_values(
        task_authority_id,
        task_principal_id,
        task_session_id,
        format!(
            "evaluation-provenance-authority-{}",
            &provenance_digest[7..23]
        ),
        format!(
            "evaluation-provenance-principal-{}",
            &provenance_digest[23..39]
        ),
        digest(format!("provenance-session|{}", provenance_seed).as_bytes()),
        grader_authority_id,
        grader_principal_id,
        grader_session_id,
    ))
}
