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
}

include!("owner_execution.rs");
