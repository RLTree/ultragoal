impl FileEvaluationExecutionLedger {
    fn publish_result(
        &mut self,
        run_sha256: impl Into<String>,
        artifact_set_sha256: impl Into<String>,
    ) -> Result<(), EvaluationLedgerError> {
        self.publish_terminal_result(run_sha256, artifact_set_sha256, b"{}".to_vec())
    }

    fn publish_terminal_result(
        &mut self,
        run_sha256: impl Into<String>,
        artifact_set_sha256: impl Into<String>,
        terminal_result: Vec<u8>,
    ) -> Result<(), EvaluationLedgerError> {
        let run_sha256 = run_sha256.into();
        let artifact_set_sha256 = artifact_set_sha256.into();
        if !super::valid_sha256(&run_sha256)
            || !super::valid_sha256(&artifact_set_sha256)
            || terminal_result.is_empty()
            || terminal_result.len() > 64 * 1024
            || serde_json::from_slice::<serde_json::Value>(&terminal_result).is_err()
        {
            return Err(EvaluationLedgerError::new(
                "evaluation-terminal-result-invalid",
            ));
        }
        self.transition(None, |_, current| {
            match current.payload.core.state.clone() {
                EvaluationLedgerState::Reserved => Ok(EvaluationLedgerState::Published {
                    run_sha256,
                    artifact_set_sha256,
                    terminal_result,
                }),
                _ => Err(EvaluationLedgerError::new(
                    "evaluation-publication-transition-refused",
                )),
            }
        })
    }
}
