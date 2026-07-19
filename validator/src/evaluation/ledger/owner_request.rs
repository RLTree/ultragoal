use super::EvaluationSpec;
use super::production_input::ProductionSpecPermit;

pub(in crate::evaluation) struct ProductionExecutionRequest<'a> {
    spec: &'a EvaluationSpec,
    input_root: std::path::PathBuf,
    ledger_root: std::path::PathBuf,
    ledger_key: [u8; 32],
    execution_session_id: String,
    artifact_root_sha256: String,
    runtime_configuration: RuntimeConfiguration,
    output: Option<crate::context::AuthorizedPath>,
}

impl<'a> ProductionExecutionRequest<'a> {
    pub(in crate::evaluation) fn new(
        spec: &'a EvaluationSpec,
        input_root: impl Into<std::path::PathBuf>,
        ledger_root: impl Into<std::path::PathBuf>,
        ledger_key: [u8; 32],
        execution_session_id: impl Into<String>,
        artifact_root_sha256: impl Into<String>,
        runtime_configuration: RuntimeConfiguration,
    ) -> Self {
        Self {
            spec,
            input_root: input_root.into(),
            ledger_root: ledger_root.into(),
            ledger_key,
            execution_session_id: execution_session_id.into(),
            artifact_root_sha256: artifact_root_sha256.into(),
            runtime_configuration,
            output: None,
        }
    }

    pub(in crate::evaluation) fn with_output(
        mut self,
        output: crate::context::AuthorizedPath,
    ) -> Self {
        self.output = Some(output);
        self
    }

    pub(in crate::evaluation) fn execute<B: FixtureEvaluationBridge>(
        self,
        bridge: &mut B,
    ) -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
        let permit = ProductionSpecPermit::issue_from_root(self.spec, &self.input_root)?;
        let binding = EvaluationExecutionBinding::new(EvaluationExecutionBindingRequest {
            live_context_id: self.spec.live_context_id().to_owned(),
            candidate_id: self.spec.candidate_id().to_owned(),
            spec_sha256: self.spec.spec_sha256().to_owned(),
            task_set_sha256: self.spec.task_set_sha256().to_owned(),
            execution_session_id: self.execution_session_id.clone(),
            execution_material_set_sha256: permit.material_set_sha256().to_owned(),
            artifact_root_sha256: self.artifact_root_sha256,
        })
        .map_err(|error| ProductionRuntimeError::new(error.code()))?;
        let mut owner =
            ExecutionOwner::prepare(&self.ledger_root, self.ledger_key, binding.clone())
                .map_err(|error| ProductionRuntimeError::new(error.code()))?;
        owner.execute_production(
            &permit,
            self.execution_session_id,
            self.runtime_configuration,
            &binding,
            self.output.as_ref(),
            bridge,
        )
    }
}
