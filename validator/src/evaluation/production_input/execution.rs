use super::super::runtime::{
    FixtureEvaluationBridge, ProductionEvaluationRun, ProductionRuntimeError, execute_production,
};
use super::super::{
    EvaluationExecutionBinding, EvaluationSpec, FileEvaluationExecutionLedger, RuntimeConfiguration,
};
use super::{ProductionEvidenceAuthority, ProductionSpecPermit};
use std::path::PathBuf;

pub(crate) struct ProductionExecutionRequest<'a> {
    spec: &'a EvaluationSpec,
    input_root: PathBuf,
    ledger_root: PathBuf,
    ledger_key: [u8; 32],
    binding: EvaluationExecutionBinding,
    authority: ProductionEvidenceAuthority,
    execution_session_id: String,
    runtime_configuration: RuntimeConfiguration,
}

impl<'a> ProductionExecutionRequest<'a> {
    pub(crate) fn new(
        spec: &'a EvaluationSpec,
        input_root: impl Into<PathBuf>,
        ledger_root: impl Into<PathBuf>,
        ledger_key: [u8; 32],
        binding: EvaluationExecutionBinding,
        authority: ProductionEvidenceAuthority,
        execution_session_id: impl Into<String>,
        runtime_configuration: RuntimeConfiguration,
    ) -> Self {
        Self {
            spec,
            input_root: input_root.into(),
            ledger_root: ledger_root.into(),
            ledger_key,
            binding,
            authority,
            execution_session_id: execution_session_id.into(),
            runtime_configuration,
        }
    }

    pub(crate) fn prepare(self) -> Result<PreparedProductionExecution<'a>, ProductionRuntimeError> {
        let permit = ProductionSpecPermit::issue(self.spec, &self.input_root, self.authority)?;
        if self.binding.live_context_id != self.spec.live_context_id()
            || self.binding.candidate_id != self.spec.candidate_id()
            || self.binding.spec_sha256 != self.spec.spec_sha256()
            || self.binding.task_set_sha256 != self.spec.task_set_sha256()
            || self.binding.execution_session_id != self.execution_session_id
            || self.binding.execution_material_set_sha256 != permit.material_set_sha256()
        {
            return Err(ProductionRuntimeError::new(
                "evaluation-production-ledger-binding-invalid",
            ));
        }
        let ledger = match FileEvaluationExecutionLedger::initialize(
            &self.ledger_root,
            self.ledger_key,
            self.binding.clone(),
        ) {
            Ok(ledger) => ledger,
            Err(error) if error.code() == "evaluation-ledger-already-exists" => {
                FileEvaluationExecutionLedger::open(
                    &self.ledger_root,
                    self.ledger_key,
                    self.binding,
                )
                .map_err(|error| ProductionRuntimeError::new(error.code()))?
            }
            Err(error) => return Err(ProductionRuntimeError::new(error.code())),
        };
        Ok(PreparedProductionExecution {
            permit,
            ledger,
            execution_session_id: self.execution_session_id,
            runtime_configuration: self.runtime_configuration,
        })
    }
}

pub(crate) struct PreparedProductionExecution<'a> {
    permit: ProductionSpecPermit<'a>,
    ledger: FileEvaluationExecutionLedger,
    execution_session_id: String,
    runtime_configuration: RuntimeConfiguration,
}

impl<'a> PreparedProductionExecution<'a> {
    pub(crate) fn execute<B: FixtureEvaluationBridge>(
        mut self,
        bridge: &mut B,
    ) -> Result<ProductionEvaluationRun, ProductionRuntimeError> {
        execute_production(
            &self.permit,
            self.execution_session_id,
            self.runtime_configuration,
            &mut self.ledger,
            bridge,
        )
    }
}
