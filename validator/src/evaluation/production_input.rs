#[path = "production_input/descriptor.rs"]
mod descriptor;
#[path = "production_input/evidence_authority.rs"]
mod evidence_authority;
#[path = "production_input/execution.rs"]
mod execution;
#[path = "production_input/material.rs"]
mod material;

pub(crate) use execution::ProductionExecutionRequest;
pub(crate) use material::AuthenticatedTaskMaterial;

use super::{EvaluationError, EvaluationSpec, EvaluationTask, TaskAudit, digest};
use descriptor::{ProtectedProductionInput, open_production_root, production_identity};
use evidence_authority::{
    AuthorizedEvidenceBinding, ProductionEvidenceAuthority, ProductionEvidenceRequest,
};
use std::cell::RefCell;
use std::collections::BTreeSet;

struct ProtectedTaskInputs {
    task_id: String,
    dataset: ProtectedProductionInput,
    scorer: ProtectedProductionInput,
    grader: ProtectedProductionInput,
}

/// A one-shot, descriptor-bound authorization to execute an already-audited
/// specification. The authority and all material identities are confined to
/// this evaluation domain; callers cannot synthesize either.
pub(crate) struct ProductionSpecPermit<'a> {
    spec: &'a EvaluationSpec,
    audit: TaskAudit,
    authority: AuthorizedEvidenceBinding,
    root_path: std::path::PathBuf,
    root: std::fs::File,
    root_identity: descriptor::ProductionInputIdentity,
    tasks: Vec<ProtectedTaskInputs>,
    material_set_sha256: String,
    issued_tasks: RefCell<BTreeSet<String>>,
}

impl<'a> ProductionSpecPermit<'a> {
    pub(super) fn issue_from_root(
        spec: &'a EvaluationSpec,
        root: impl AsRef<std::path::Path>,
    ) -> Result<Self, EvaluationError> {
        let evidence = derive_root_evidence(spec, root.as_ref())?;
        Self::issue(
            spec,
            root,
            ProductionEvidenceAuthority::issue(spec, evidence)?,
        )
    }

    fn issue(
        spec: &'a EvaluationSpec,
        root: impl AsRef<std::path::Path>,
        authority: ProductionEvidenceAuthority,
    ) -> Result<Self, EvaluationError> {
        let authority = authority.consume(spec)?;
        let audit = spec.audit(spec.live_context_id(), spec.candidate_id());
        if !audit.eligible() {
            return Err(EvaluationError::new(
                "evaluation-production-spec-ineligible",
            ));
        }
        let requested_root = root.as_ref().to_path_buf();
        let root = open_production_root(&requested_root)?;
        let root_path = std::fs::canonicalize(&requested_root)
            .map_err(|_| EvaluationError::new("evaluation-input-root-open-failed"))?;
        let root_identity = production_identity(&root)?;
        if production_identity(&open_production_root(&root_path)?)? != root_identity {
            return Err(EvaluationError::new("evaluation-input-root-changed"));
        }
        let mut tasks = Vec::with_capacity(spec.tasks.len());
        for task in &spec.tasks {
            let dataset = ProtectedProductionInput::capture(
                &root,
                task.dataset.relative_path(),
                task.dataset.digest_sha256(),
                Some(task.dataset.byte_length),
            )?;
            let scorer = ProtectedProductionInput::capture(
                &root,
                &format!("scorers/{}", task.scorer_id),
                &task.scorer_digest_sha256,
                None,
            )?;
            let grader = ProtectedProductionInput::capture_observed(
                &root,
                &format!("graders/{}.json", task.task_id),
            )?;
            tasks.push(ProtectedTaskInputs {
                task_id: task.task_id.clone(),
                dataset,
                scorer,
                grader,
            });
        }
        let material_set_sha256 = material_set_sha256(&tasks);
        let permit = Self {
            spec,
            audit,
            authority,
            root_path,
            root,
            root_identity,
            tasks,
            material_set_sha256,
            issued_tasks: RefCell::new(BTreeSet::new()),
        };
        permit.revalidate()?;
        Ok(permit)
    }

    pub(crate) fn spec(&self) -> &EvaluationSpec {
        self.spec
    }
    pub(crate) fn audit(&self) -> &TaskAudit {
        &self.audit
    }
    pub(crate) fn material_set_sha256(&self) -> &str {
        &self.material_set_sha256
    }

    pub(crate) fn take_task_material(
        &self,
        task: &EvaluationTask,
    ) -> Result<AuthenticatedTaskMaterial, EvaluationError> {
        let protected = self
            .tasks
            .iter()
            .find(|entry| entry.task_id == task.task_id)
            .ok_or_else(|| EvaluationError::new("evaluation-task-material-missing"))?;
        if !self.issued_tasks.borrow_mut().insert(task.task_id.clone()) {
            return Err(EvaluationError::new("evaluation-task-material-replayed"));
        }
        self.revalidate()?;
        AuthenticatedTaskMaterial::issue(task, protected, &self.authority)
    }

    pub(crate) fn revalidate(&self) -> Result<(), EvaluationError> {
        let named_root = open_production_root(&self.root_path)?;
        if production_identity(&self.root)? != self.root_identity
            || production_identity(&named_root)? != self.root_identity
        {
            return Err(EvaluationError::new("evaluation-input-root-changed"));
        }
        for task in &self.tasks {
            task.dataset.revalidate(&self.root)?;
            task.scorer.revalidate(&self.root)?;
            task.grader.revalidate(&self.root)?;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn test_issue(
        spec: &'a EvaluationSpec,
        root: impl AsRef<std::path::Path>,
    ) -> Result<Self, EvaluationError> {
        Self::issue(spec, root, ProductionEvidenceAuthority::test_issue(spec))
    }
}

include!("production_input/root_evidence.rs");

fn material_set_sha256(tasks: &[ProtectedTaskInputs]) -> String {
    let mut rows = tasks
        .iter()
        .map(|task| {
            format!(
                "{}|{}|{}|{}",
                task.task_id,
                task.dataset.digest_sha256(),
                task.scorer.digest_sha256(),
                task.grader.digest_sha256()
            )
        })
        .collect::<Vec<_>>();
    rows.sort_unstable();
    digest(rows.join("\n").as_bytes())
}
