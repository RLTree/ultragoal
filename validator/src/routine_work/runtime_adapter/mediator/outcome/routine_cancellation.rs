use super::*;

#[derive(Clone)]
pub(crate) struct RoutineCancellation {
    pub(crate) cancelled: Arc<AtomicBool>,
}

impl RoutineCancellation {
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    #[cfg(test)]
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RoutineMediatorStatus {
    CompleteNoOp,
    CompleteExecution,
    IncompleteExecution,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum RoutineNodeDisposition {
    Executed,
    Reused,
    Failed,
    DependencyFailed,
    Cancelled,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(crate) struct RoutineNodeMediation {
    pub(crate) intent_id: String,
    pub(crate) node_id: String,
    pub(crate) plan_order: usize,
    pub(crate) disposition: RoutineNodeDisposition,
    pub(crate) result_artifact_sha256: Option<String>,
    pub(crate) failure_code: Option<String>,
}

impl RoutineNodeMediation {
    pub(crate) fn node_id(&self) -> &str {
        &self.node_id
    }

    pub(crate) fn disposition(&self) -> RoutineNodeDisposition {
        self.disposition
    }

    pub(crate) fn result_artifact_sha256(&self) -> Option<&str> {
        self.result_artifact_sha256.as_deref()
    }

    pub(crate) fn failure_code(&self) -> Option<&str> {
        self.failure_code.as_deref()
    }
}

#[derive(Debug)]
#[must_use = "mediation results must be reconciled by their root caller"]
pub(crate) struct RoutineMediationResult {
    pub(crate) request_id: Option<String>,
    pub(crate) protocol_id: Option<String>,
    pub(crate) status: RoutineMediatorStatus,
    pub(crate) nodes: Vec<RoutineNodeMediation>,
    pub(crate) recovery_marker: Option<String>,
    pub(crate) support_limit: &'static str,
}
