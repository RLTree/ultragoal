/// The only production custody handle. The raw journal and every mutation are
/// kept below this leaf; callers receive one complete production operation.
struct ExecutionOwner(FileEvaluationExecutionLedger);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExecutionOwnerError {
    code: &'static str,
}

impl ExecutionOwnerError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    fn code(&self) -> &'static str {
        self.code
    }
}

impl ExecutionOwner {
    fn prepare(
        root: impl AsRef<Path>,
        key: [u8; 32],
        binding: EvaluationExecutionBinding,
    ) -> Result<Self, ExecutionOwnerError> {
        let root = root.as_ref();
        let ledger = match FileEvaluationExecutionLedger::initialize(root, key, binding.clone()) {
            Ok(ledger) => ledger,
            Err(error) if error.code() == "evaluation-ledger-already-exists" => {
                FileEvaluationExecutionLedger::open(root, key, binding)
                    .map_err(|error| ExecutionOwnerError::new(error.code()))?
            }
            Err(error) => return Err(ExecutionOwnerError::new(error.code())),
        };
        Ok(Self(ledger))
    }

    fn publish_terminal_result(
        &mut self,
        run_sha256: &str,
        artifact_set_sha256: String,
        terminal_result: Vec<u8>,
    ) -> Result<(), ProductionRuntimeError> {
        self.0
            .publish_terminal_result(run_sha256, artifact_set_sha256, terminal_result)
            .map_err(|error| ProductionRuntimeError::new(error.code()))
    }
}

include!("owner_execution.rs");
