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

/// Opaque root authority consumed by one routine mediation attempt.
///
/// Production construction is intentionally absent. The canonical root issuer
/// remains root-owned wiring; this module only verifies and consumes grants.
#[must_use = "a root grant must be consumed once or explicitly discarded"]
pub(crate) struct RoutineRootGrant {
    pub(crate) grant_id: String,
    pub(crate) session_id: String,
    pub(crate) request_id: String,
    pub(crate) protocol_id: String,
    pub(crate) context_id: String,
    pub(crate) candidate_id: String,
    pub(crate) plan_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) allowed_output_scopes: Vec<RepoPath>,
    pub(crate) recovery_for: Option<String>,
    pub(crate) seal: String,
    pub(crate) durable: Option<Arc<dyn DurableAttemptAuthority>>,
}

#[cfg(test)]
impl RoutineRootGrant {
    pub(crate) fn test_issue(
        request: &RoutineEffectRequest,
        session_id: impl Into<String>,
        recovery_for: Option<String>,
    ) -> Self {
        let session_id = session_id.into();
        let mut allowed_output_scopes = request
            .intents
            .iter()
            .flat_map(|intent| intent.declared_output_scopes().iter().cloned())
            .collect::<Vec<_>>();
        allowed_output_scopes.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        allowed_output_scopes.dedup();
        let mut grant = Self {
            grant_id: String::new(),
            session_id,
            request_id: request.request_id.clone(),
            protocol_id: request.protocol_id.clone(),
            context_id: request.binding.context_id().to_owned(),
            candidate_id: request.binding.candidate_id().to_owned(),
            plan_id: request.plan_id.clone(),
            snapshot_id: request.snapshot_id.clone(),
            allowed_output_scopes,
            recovery_for,
            seal: String::new(),
            durable: None,
        };
        grant.grant_id = super::super::grant_identity(&grant).expect("test grant identity");
        grant.seal = super::super::grant_seal(&grant).expect("test grant seal");
        grant
    }

    pub(crate) fn test_duplicate(&self) -> Self {
        Self {
            grant_id: self.grant_id.clone(),
            session_id: self.session_id.clone(),
            request_id: self.request_id.clone(),
            protocol_id: self.protocol_id.clone(),
            context_id: self.context_id.clone(),
            candidate_id: self.candidate_id.clone(),
            plan_id: self.plan_id.clone(),
            snapshot_id: self.snapshot_id.clone(),
            allowed_output_scopes: self.allowed_output_scopes.clone(),
            recovery_for: self.recovery_for.clone(),
            seal: self.seal.clone(),
            durable: None,
        }
    }

    pub(crate) fn test_recovery_marker(&self) -> String {
        super::super::recovery_identity(&self.grant_id, &self.protocol_id, &self.request_id)
    }

    pub(crate) fn test_with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = request_id.into();
        self
    }

    pub(crate) fn test_with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = session_id.into();
        self
    }

    pub(crate) fn test_with_context_id(mut self, context_id: impl Into<String>) -> Self {
        self.context_id = context_id.into();
        self
    }

    pub(crate) fn test_with_plan_id(mut self, plan_id: impl Into<String>) -> Self {
        self.plan_id = plan_id.into();
        self
    }

    pub(crate) fn test_with_scopes(mut self, scopes: Vec<RepoPath>) -> Self {
        self.allowed_output_scopes = scopes;
        self
    }

    pub(crate) fn test_with_seal(mut self, seal: impl Into<String>) -> Self {
        self.seal = seal.into();
        self
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
