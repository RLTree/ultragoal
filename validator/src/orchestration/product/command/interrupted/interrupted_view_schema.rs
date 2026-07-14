const INTERRUPTED_VIEW_SCHEMA: &str = "OrchestrationInterruptedRecoveryView-v1";
const INTERRUPTED_VIEW_DOMAIN: &[u8] =
    b"harness-ultragoal/orchestration-interrupted-recovery-view/v1\0";
const MAX_INTERRUPTED_VIEW_COMMITMENT_BYTES: usize = 16 * 1024 * 1024 - 4096;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterruptedRecoveryRequest {
    pub expected_prior_head: JournalHead,
    pub expected_event_id: String,
    pub recovered_binding: Binding,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterruptedRecoveryView {
    pub schema_version: String,
    pub workspace_identity: String,
    pub prior_journal_head_identity: String,
    pub prior_head: JournalHead,
    pub prospective_journal_head_identity: String,
    pub prospective_head: JournalHead,
    pub snapshot_id: String,
    pub snapshot: ProductSnapshot,
    pub root_action_request: RootActionRequest,
    #[serde(skip)]
    authority: InterruptedViewAuthority,
}

/// Process-local construction authority for one exact complete interrupted
/// preview. Serialized bytes always deserialize without this authority and must
/// be replaced by a fresh trusted inspection before root can issue a permit.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct InterruptedViewAuthority {
    committed_view_id: Option<String>,
}

impl InterruptedViewAuthority {
    fn issue(view_id: &str) -> Self {
        Self {
            committed_view_id: Some(view_id.to_owned()),
        }
    }

    fn matches(&self, view_id: &str) -> bool {
        self.committed_view_id.as_deref() == Some(view_id)
    }
}

impl InterruptedRecoveryView {
    pub fn validate_action(
        &self,
        workspace: &ProductWorkspace,
        action: &RootActionRequest,
    ) -> Result<(), ProductError> {
        self.require_authority()?;
        let expected = self.rederive_current_action(workspace)?;
        if action != &expected || action != &self.root_action_request {
            return Err(ProductError::AuthorityInvalid);
        }
        action.validate_for(workspace)?;
        let final_expected = self.rederive_current_action(workspace)?;
        if final_expected != expected || action != &final_expected {
            return Err(ProductError::ConcurrentUpdate);
        }
        Ok(())
    }

    fn require_authority(&self) -> Result<(), ProductError> {
        if self.schema_version != INTERRUPTED_VIEW_SCHEMA {
            return Err(ProductError::AuthorityInvalid);
        }
        let view_id = interrupted_view_id(self)?;
        if !self.authority.matches(&view_id) {
            return Err(ProductError::AuthorityInvalid);
        }
        let snapshot_id = self.snapshot.snapshot_id()?;
        if self.prior_journal_head_identity != journal_head_identity(&self.prior_head)?
            || self.prospective_journal_head_identity
                != journal_head_identity(&self.prospective_head)?
            || self.snapshot_id != snapshot_id
            || self.root_action_request.snapshot_id != snapshot_id
            || self.snapshot.journal_head != self.prospective_head
            || self.snapshot.binding != self.prospective_head.binding
            || self.snapshot.event_count as u64 != self.prospective_head.event_count
        {
            return Err(ProductError::AuthorityInvalid);
        }
        Ok(())
    }

    fn rederive_current_action(
        &self,
        workspace: &ProductWorkspace,
    ) -> Result<RootActionRequest, ProductError> {
        workspace.verify()?;
        if self.workspace_identity != workspace.identity() {
            return Err(ProductError::AuthorityInvalid);
        }
        let prospective = FileJournal::inspect_interrupted_append(
            workspace.root(),
            &self.prior_head,
            &self.prospective_head.last_event_id,
            &self.prospective_head.binding,
        )
        .map_err(ProductError::from)?;
        if prospective.head != self.prospective_head
            || journal_head_identity(&prospective.head)? != self.prospective_journal_head_identity
        {
            return Err(ProductError::ConcurrentUpdate);
        }
        let target = recovery_target(&prospective, &self.snapshot)?;
        if target != self.root_action_request.target {
            return Err(ProductError::AuthorityInvalid);
        }
        let action = RootActionRequest::new(RootActionRequestDefinition {
            operation: RootOperation::Recover,
            reason: RootActionReason::InterruptedPublication,
            authority_binding: self.prior_head.binding.clone(),
            workspace_identity: workspace.identity().to_owned(),
            expected_head: self.prior_head.clone(),
            snapshot_id: self.snapshot_id.clone(),
            target,
        })?;
        if action != self.root_action_request {
            return Err(ProductError::AuthorityInvalid);
        }
        workspace.verify()?;
        Ok(action)
    }
}
