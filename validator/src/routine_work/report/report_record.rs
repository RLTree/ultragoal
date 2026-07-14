use super::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipReason {
    Cancelled,
    DependencyFailed,
    CapabilityUnavailable,
    OperatorExcluded,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReportDisposition {
    Executed(ExecutedWork),
    Reused(VerifiedReuse),
    Skipped(SkipReason),
    Failed { cause_code: String },
}

#[derive(Debug, Eq, PartialEq)]
pub struct ReportRecord {
    pub(crate) node_id: String,
    pub(crate) disposition: ReportDisposition,
}

impl ReportRecord {
    pub fn new(node_id: impl Into<String>, disposition: ReportDisposition) -> Self {
        Self {
            node_id: node_id.into(),
            disposition,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportStatus {
    CompleteExecution,
    IncompleteExecution,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RoutineReport {
    pub(crate) report_id: String,
    pub(crate) binding_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) result_scope: String,
    pub(crate) selected: Vec<String>,
    pub(crate) executed: Vec<String>,
    pub(crate) reused: Vec<String>,
    pub(crate) skipped: BTreeMap<String, SkipReason>,
    pub(crate) failed: BTreeMap<String, String>,
    pub(crate) status: ReportStatus,
    pub(crate) support_limit: &'static str,
}

#[derive(Serialize)]
pub(crate) struct ReportPayload<'a> {
    pub(crate) binding_id: &'a str,
    pub(crate) context_id: &'a str,
    pub(crate) candidate_id: &'a str,
    pub(crate) plan_id: &'a str,
    pub(crate) result_scope: &'a str,
    pub(crate) selected: &'a [String],
    pub(crate) executed: &'a [String],
    pub(crate) reused: &'a [String],
    pub(crate) skipped: &'a BTreeMap<String, SkipReason>,
    pub(crate) failed: &'a BTreeMap<String, String>,
    pub(crate) status: ReportStatus,
    pub(crate) support_limit: &'static str,
}

impl RoutineReport {
    pub fn report_id(&self) -> &str {
        &self.report_id
    }
    pub fn context_id(&self) -> &str {
        &self.context_id
    }
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }
    pub fn selected(&self) -> &[String] {
        &self.selected
    }
    pub fn executed(&self) -> &[String] {
        &self.executed
    }
    pub fn reused(&self) -> &[String] {
        &self.reused
    }
    pub fn skipped(&self) -> &BTreeMap<String, SkipReason> {
        &self.skipped
    }
    pub fn failed(&self) -> &BTreeMap<String, String> {
        &self.failed
    }
    pub fn status(&self) -> ReportStatus {
        self.status
    }
    pub fn result_scope(&self) -> &str {
        &self.result_scope
    }
    pub fn support_limit(&self) -> &'static str {
        self.support_limit
    }
}
