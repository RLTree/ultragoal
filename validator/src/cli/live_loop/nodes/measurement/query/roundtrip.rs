use crate::cli::observe::command::ObserveOperation;

pub(in crate::cli::live_loop::nodes::measurement::observation) const EXPLAIN_TIMEOUT_MS: u64 =
    1_000;
pub(in crate::cli::live_loop::nodes::measurement::observation) const LIVE_BACKEND_QUERY_TIMEOUT_MS:
    u64 = 3_000;
pub(in crate::cli::live_loop::nodes::measurement::observation) const TRACE_QUERY_TIMEOUT_MS: u64 =
    5_000;
pub(in crate::cli::live_loop::nodes::measurement::observation) const METRICS_QUERY_TIMEOUT_MS: u64 =
    10_000;
pub(in crate::cli::live_loop::nodes::measurement::observation) const ROW_LIMIT: usize = 100;
pub(in crate::cli::live_loop::nodes::measurement::observation) const BYTE_LIMIT: usize = 262_144;
pub(in crate::cli::live_loop::nodes::measurement::observation) const RECEIPT_DIR: &str =
    "validation_artifacts/observability/live-loop/commands";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cli::live_loop::nodes::measurement::observation) enum RoundtripQuery {
    Logs,
    Metrics,
    Traces,
    ExplainFailure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::cli::live_loop::nodes::measurement::observation) enum LiveQueryRoundtrip {
    Logs,
    Metrics,
    Traces,
}

#[derive(Clone, Copy)]
pub(in crate::cli::live_loop::nodes::measurement::observation) enum BackendState {
    Ready,
    Unavailable(super::super::super::live_backend::LiveBackend),
}

impl From<LiveQueryRoundtrip> for RoundtripQuery {
    fn from(value: LiveQueryRoundtrip) -> Self {
        match value {
            LiveQueryRoundtrip::Logs => Self::Logs,
            LiveQueryRoundtrip::Metrics => Self::Metrics,
            LiveQueryRoundtrip::Traces => Self::Traces,
        }
    }
}

impl LiveQueryRoundtrip {
    pub(in crate::cli::live_loop::nodes::measurement::observation) fn operation(
        self,
    ) -> ObserveOperation {
        RoundtripQuery::from(self).operation()
    }

    pub(in crate::cli::live_loop::nodes::measurement::observation) fn receipt_suffix(
        self,
    ) -> &'static str {
        RoundtripQuery::from(self).receipt_suffix()
    }

    pub(in crate::cli::live_loop::nodes::measurement::observation) fn timeout_ms(self) -> u64 {
        RoundtripQuery::from(self).timeout_ms()
    }

    pub(in crate::cli::live_loop::nodes::measurement::observation) fn query_kind(
        self,
    ) -> crate::cli::observe::query::QueryKind {
        match self {
            Self::Logs => crate::cli::observe::query::QueryKind::Logs,
            Self::Metrics => crate::cli::observe::query::QueryKind::Metrics,
            Self::Traces => crate::cli::observe::query::QueryKind::Traces,
        }
    }
}

impl RoundtripQuery {
    pub(in crate::cli::live_loop::nodes::measurement::observation) fn operation(
        self,
    ) -> ObserveOperation {
        match self {
            Self::Logs => ObserveOperation::LogsQuery,
            Self::Metrics => ObserveOperation::MetricsQuery,
            Self::Traces => ObserveOperation::TracesQuery,
            Self::ExplainFailure => ObserveOperation::ExplainFailure,
        }
    }

    pub(in crate::cli::live_loop::nodes::measurement::observation) fn receipt_suffix(
        self,
    ) -> &'static str {
        match self {
            Self::Logs => "logs-query",
            Self::Metrics => "metrics-query",
            Self::Traces => "traces-query",
            Self::ExplainFailure => "explain-failure",
        }
    }

    pub(in crate::cli::live_loop::nodes::measurement::observation) fn timeout_ms(self) -> u64 {
        match self {
            Self::Logs => LIVE_BACKEND_QUERY_TIMEOUT_MS,
            Self::Metrics => METRICS_QUERY_TIMEOUT_MS,
            Self::Traces => TRACE_QUERY_TIMEOUT_MS,
            Self::ExplainFailure => EXPLAIN_TIMEOUT_MS,
        }
    }
}
