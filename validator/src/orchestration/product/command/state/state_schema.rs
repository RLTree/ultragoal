const STATE_SCHEMA: &str = "OrchestrationStateOverlay-v1";
const UNTRUSTED_STATE_ID: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";
const MAX_STATE_COMMITMENT_BYTES: usize = 16 * 1024 * 1024 - 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrchestrationStateRequest {
    pub expected_head: crate::orchestration::JournalHead,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationStateView {
    pub schema_version: String,
    pub state_id: String,
    pub workspace_identity: String,
    pub journal_head_identity: String,
    pub snapshot_id: String,
    pub snapshot: ProductSnapshot,
    pub findings: Vec<OrchestrationFinding>,
    pub root_action_requests: Vec<RootActionRequest>,
    #[serde(skip)]
    authority: StateAuthority,
}

/// Process-local construction authority for one exact complete view. It is
/// deliberately absent from serialized output and defaults to untrusted on
/// deserialization, so wire bytes can never recreate an authoritative view.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct StateAuthority {
    committed_state_id: Option<String>,
}

impl StateAuthority {
    fn issue(state_id: &str) -> Self {
        Self {
            committed_state_id: Some(state_id.to_owned()),
        }
    }

    fn matches(&self, state_id: &str) -> bool {
        self.committed_state_id.as_deref() == Some(state_id)
    }
}

impl OrchestrationStateView {
    pub(super) fn require_authority(&self) -> Result<(), ProductError> {
        if self.schema_version != STATE_SCHEMA || !self.authority.matches(&self.state_id) {
            return Err(ProductError::AuthorityInvalid);
        }
        if self.state_id != state_id(self)?
            || self.snapshot_id != self.snapshot.snapshot_id()?
            || self.snapshot.binding != self.snapshot.journal_head.binding
            || self.journal_head_identity != journal_head_identity(&self.snapshot.journal_head)?
        {
            return Err(ProductError::AuthorityInvalid);
        }
        Ok(())
    }

    pub fn revalidate(&self, workspace: &ProductWorkspace) -> Result<(), ProductError> {
        self.require_authority()?;
        workspace.verify()?;
        let current = FileJournal::open(workspace.root())
            .and_then(|journal| journal.inspect())
            .map_err(ProductError::from)?;
        if self.workspace_identity != workspace.identity()
            || self.journal_head_identity != journal_head_identity(&current.head)?
            || self.snapshot.journal_head != current.head
        {
            return Err(ProductError::ConcurrentUpdate);
        }
        workspace.verify()
    }

    /// Validates that a serialized request is both current and one of the exact
    /// requests derived from this authenticated state view.
    pub fn validate_action(
        &self,
        workspace: &ProductWorkspace,
        action: &RootActionRequest,
    ) -> Result<(), ProductError> {
        self.revalidate(workspace)?;
        action.validate_for(workspace)?;
        if self.workspace_identity != workspace.identity()
            || self.journal_head_identity != action.journal_head_identity
            || self.snapshot.journal_head != action.expected_head
            || !self.root_action_requests.contains(action)
        {
            return Err(ProductError::AuthorityInvalid);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextDisposition {
    NoAction,
    ContinueDependencyClosedWork,
    RootAuthorityRequired,
    NoLegalRoute,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationNext {
    pub schema_version: String,
    pub state_id: String,
    pub disposition: NextDisposition,
    pub ready_node: Option<String>,
    pub finding_id: Option<String>,
    pub root_action_request: Option<RootActionRequest>,
}

pub fn inspect(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    request: &OrchestrationStateRequest,
) -> Result<OrchestrationStateView, ProductError> {
    let snapshot = query(
        context,
        workspace,
        &QueryRequest {
            expected_head: request.expected_head.clone(),
            tick: request.tick,
            live_workers: request.live_workers.clone(),
        },
    )?;
    let snapshot_id = snapshot.snapshot_id()?;
    let pending = pending_effect_leases(workspace, &snapshot.journal_head)?;
    let findings = build_findings(&snapshot, &pending)?;
    let root_action_requests = actions(context, workspace, &snapshot, &snapshot_id, &pending)?;
    let journal_head_identity = journal_head_identity(&snapshot.journal_head)?;
    let mut view = OrchestrationStateView {
        schema_version: STATE_SCHEMA.to_owned(),
        state_id: String::new(),
        workspace_identity: workspace.identity().to_owned(),
        journal_head_identity,
        snapshot_id,
        snapshot,
        findings,
        root_action_requests,
        authority: StateAuthority::default(),
    };
    view.state_id = state_id(&view)?;
    view.authority = StateAuthority::issue(&view.state_id);
    view.revalidate(workspace)?;
    Ok(view)
}
