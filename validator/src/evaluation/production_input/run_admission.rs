use super::super::ledger::ProductionExecutionRequest;
use super::super::runtime::{
    FixtureEvaluationBridge, ProductionEvaluationRun, ProductionRuntimeError,
};
use super::super::{EvaluationError, EvaluationSpec, RuntimeConfiguration, digest};
use crate::context::LiveContext;
use sha2::{Digest, Sha256};

/// The only production admission for an evaluation run. It derives all mutable
/// custody inputs from one current workspace context; callers cannot choose a
/// ledger path, signing key, session, or recovery authority.
pub(crate) struct EvaluationRunAdmission<'a> {
    spec: &'a EvaluationSpec,
    root: std::path::PathBuf,
    ledger_root: std::path::PathBuf,
    ledger_key: [u8; 32],
    session_id: String,
    artifact_root_sha256: String,
    output: crate::context::AuthorizedPath,
}

impl<'a> EvaluationRunAdmission<'a> {
    pub(crate) fn issue(
        context: &LiveContext,
        spec: &'a EvaluationSpec,
        output_relative: &str,
    ) -> Result<Self, EvaluationError> {
        if context.revalidate().is_err()
            || spec.live_context_id() != context.context_id()
            || spec.candidate_id() != candidate_id(context)?
        {
            return Err(EvaluationError::new("evaluation-run-admission-stale"));
        }
        let root = context.worktree_root().to_path_buf();
        let output = context
            .authorize_path(
                output_relative,
                crate::cli::successor::EffectClass::WorkspaceWrite,
            )
            .map_err(|_| EvaluationError::new("evaluation-run-output-unavailable"))?;
        if output
            .canonical_path
            .parent()
            .is_none_or(|parent| !parent.is_dir())
        {
            return Err(EvaluationError::new("evaluation-run-output-unavailable"));
        }
        let identity = digest(
            format!(
                "evaluation-run-admission-v1|{}|{}|{}",
                context.context_id(),
                spec.candidate_id(),
                spec.spec_sha256()
            )
            .as_bytes(),
        );
        let ledger_root = root.join(".ultragoal/evaluation").join(&identity[7..]);
        std::fs::create_dir_all(&ledger_root)
            .map_err(|_| EvaluationError::new("evaluation-run-ledger-root-create-failed"))?;
        let mut key = [0_u8; 32];
        key.copy_from_slice(&Sha256::digest(format!("key|{identity}").as_bytes()));
        Ok(Self {
            spec,
            root,
            ledger_root,
            ledger_key: key,
            session_id: digest(format!("session|{identity}").as_bytes()),
            artifact_root_sha256: digest(format!("artifact-root|{identity}").as_bytes()),
            output,
        })
    }

    pub(crate) fn execute<B: FixtureEvaluationBridge>(
        self,
        bridge: &mut B,
    ) -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
        ProductionExecutionRequest::new(
            self.spec,
            self.root,
            self.ledger_root,
            self.ledger_key,
            self.session_id,
            self.artifact_root_sha256,
            RuntimeConfiguration::all_unknown(),
        )
        .with_output(self.output)
        .execute(bridge)
    }
}

fn candidate_id(context: &LiveContext) -> Result<String, EvaluationError> {
    serde_json::to_vec(context.candidate())
        .map(|bytes| format!("sha256:{:x}", Sha256::digest(bytes)))
        .map_err(|_| EvaluationError::new("evaluation-run-admission-candidate-invalid"))
}
