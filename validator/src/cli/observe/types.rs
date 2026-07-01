use std::path::PathBuf;

pub(crate) const LAW_ID: &str = "full-local-observability-stack-integration-non-opaque-failure";
pub(crate) const CHECK_ID: &str = LAW_ID;
pub(crate) const CLAIM_ID: &str = "gate-92-observability-control-plane";
pub(crate) const RECEIPT_SCHEMA: &str = "harness-ultragoal.observability-receipt.v1";
pub(crate) const QUERY_SCHEMA: &str = "harness-ultragoal.observability-query-result.v1";
pub(crate) const EVENT_SCHEMA: &str = "harness-ultragoal.observability-event.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ObserveOperation {
    StackUp,
    StackHealth,
    StackSmoke,
    StackDown,
    StackGcPlan,
    StackGcDryRun,
    StackGcApply,
    LogsQuery,
    MetricsQuery,
    TracesQuery,
    Snapshot,
    Prove,
    ExplainFailure,
    ExplainClaim,
    ExplainCheck,
    ExplainLaw,
}

impl ObserveOperation {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::StackUp => "observe.stack.up",
            Self::StackHealth => "observe.stack.health",
            Self::StackSmoke => "observe.stack.smoke",
            Self::StackDown => "observe.stack.down",
            Self::StackGcPlan => "observe.stack.gc.plan",
            Self::StackGcDryRun => "observe.stack.gc.dry-run",
            Self::StackGcApply => "observe.stack.gc.apply",
            Self::LogsQuery => "observe.logs.query",
            Self::MetricsQuery => "observe.metrics.query",
            Self::TracesQuery => "observe.traces.query",
            Self::Snapshot => "observe.snapshot",
            Self::Prove => "observe.prove",
            Self::ExplainFailure => "observe.explain-failure",
            Self::ExplainClaim => "observe.explain-claim",
            Self::ExplainCheck => "observe.explain-check",
            Self::ExplainLaw => "observe.explain-law",
        }
    }

    pub(crate) fn subcommand(self) -> &'static str {
        match self {
            Self::StackUp | Self::StackHealth | Self::StackSmoke | Self::StackDown => "stack",
            Self::StackGcPlan | Self::StackGcDryRun | Self::StackGcApply => "stack.gc",
            Self::LogsQuery => "logs",
            Self::MetricsQuery => "metrics",
            Self::TracesQuery => "traces",
            Self::Snapshot => "snapshot",
            Self::Prove => "prove",
            Self::ExplainFailure => "explain-failure",
            Self::ExplainClaim => "explain-claim",
            Self::ExplainCheck => "explain-check",
            Self::ExplainLaw => "explain-law",
        }
    }

    pub(crate) fn receipt_rel(self) -> PathBuf {
        let name = self.id().replace('.', "-");
        PathBuf::from(format!("validation_artifacts/observability/{name}.json"))
    }
}

#[derive(Debug)]
pub(crate) struct ObserveCommand {
    pub(crate) operation: ObserveOperation,
    pub(crate) receipt: Option<PathBuf>,
    pub(crate) query: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) correlation_id: Option<String>,
    pub(crate) claim_id: Option<String>,
    pub(crate) check_id: Option<String>,
    pub(crate) law_id: Option<String>,
    pub(crate) row_limit: usize,
    pub(crate) byte_limit: usize,
    pub(crate) timeout_ms: u64,
}

impl ObserveCommand {
    pub(crate) fn receipt_rel(&self) -> PathBuf {
        self.receipt
            .clone()
            .unwrap_or_else(|| self.operation.receipt_rel())
    }
}
