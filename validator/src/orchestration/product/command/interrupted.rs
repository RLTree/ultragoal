use super::super::context::ReadOnlySink;
use super::super::snapshot::snapshot;
use super::super::{
    PermitTarget, ProductContext, ProductError, ProductSnapshot, ProductWorkspace, RootOperation,
    journal_head_identity,
};
use super::{RootActionReason, RootActionRequest};
use crate::orchestration::{
    Binding, FileJournal, JournalHead, JournalSnapshot, OrchestrationError, Orchestrator,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{self, Write};

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
        let action = RootActionRequest::new(
            RootOperation::Recover,
            RootActionReason::InterruptedPublication,
            self.prior_head.binding.clone(),
            workspace.identity().to_owned(),
            self.prior_head.clone(),
            self.snapshot_id.clone(),
            target,
        )?;
        if action != self.root_action_request {
            return Err(ProductError::AuthorityInvalid);
        }
        workspace.verify()?;
        Ok(action)
    }
}

/// Read-only preview of one exact interrupted append. The returned request is
/// not a root permit and this function never publishes the recovered head.
pub fn inspect_interrupted(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    request: &InterruptedRecoveryRequest,
) -> Result<InterruptedRecoveryView, ProductError> {
    workspace.verify()?;
    let prospective = FileJournal::inspect_interrupted_append(
        workspace.root(),
        &request.expected_prior_head,
        &request.expected_event_id,
        &request.recovered_binding,
    )
    .map_err(ProductError::from)?;
    if prospective.head.binding != request.recovered_binding {
        return Err(ProductError::StaleCandidate);
    }
    let effective_context = ProductContext::new(
        context.graph.clone(),
        context.policy.clone(),
        request.recovered_binding.clone(),
        context.root.clone(),
    );
    let preview = Orchestrator::restart_verified_snapshot(
        effective_context.graph.clone(),
        effective_context.policy.clone(),
        effective_context.root.clone(),
        prospective.clone(),
        ReadOnlySink,
    )
    .map_err(ProductError::from)?;
    let current = snapshot(&preview, request.tick, &request.live_workers)?;
    require_recoverable(&current)?;
    let target = recovery_target(&prospective, &current)?;
    let snapshot_id = current.snapshot_id()?;
    let action = RootActionRequest::new(
        RootOperation::Recover,
        RootActionReason::InterruptedPublication,
        context.binding().clone(),
        workspace.identity().to_owned(),
        request.expected_prior_head.clone(),
        snapshot_id.clone(),
        target,
    )?;
    workspace.verify()?;
    let mut view = InterruptedRecoveryView {
        schema_version: INTERRUPTED_VIEW_SCHEMA.to_owned(),
        workspace_identity: workspace.identity().to_owned(),
        prior_journal_head_identity: journal_head_identity(&request.expected_prior_head)?,
        prior_head: request.expected_prior_head.clone(),
        prospective_journal_head_identity: journal_head_identity(&prospective.head)?,
        prospective_head: prospective.head,
        snapshot_id,
        snapshot: current,
        root_action_request: action,
        authority: InterruptedViewAuthority::default(),
    };
    let view_id = interrupted_view_id(&view)?;
    view.authority = InterruptedViewAuthority::issue(&view_id);
    view.validate_action(workspace, &view.root_action_request)?;
    Ok(view)
}

fn recovery_target(
    prospective: &JournalSnapshot,
    snapshot: &ProductSnapshot,
) -> Result<PermitTarget, ProductError> {
    let event = prospective
        .log
        .events()
        .last()
        .ok_or(ProductError::UnknownOperation)?;
    if event.event_id != prospective.head.last_event_id {
        return Err(ProductError::AuthorityInvalid);
    }
    let lease_id = event.event.lease_id().map(str::to_owned);
    let result_commitment_id = lease_id.as_ref().and_then(|lease| {
        snapshot
            .commitments
            .get(lease)
            .map(|commitment| commitment.result_commitment_id.clone())
    });
    let target = PermitTarget {
        lease_id,
        result_commitment_id,
        operation_id: Some(prospective.head.last_event_id.clone()),
        recovered_binding: Some(prospective.head.binding.clone()),
    };
    let mut snapshot_target = target.clone();
    snapshot_target.operation_id = None;
    snapshot.require_target(&snapshot_target)?;
    Ok(target)
}

fn interrupted_view_id(view: &InterruptedRecoveryView) -> Result<String, ProductError> {
    #[derive(Serialize)]
    struct Commitment<'a> {
        schema_version: &'a str,
        workspace_identity: &'a str,
        prior_journal_head_identity: &'a str,
        prior_head: &'a JournalHead,
        prospective_journal_head_identity: &'a str,
        prospective_head: &'a JournalHead,
        snapshot_id: &'a str,
        snapshot: &'a ProductSnapshot,
        target: &'a PermitTarget,
        action: &'a RootActionRequest,
    }

    let mut writer = BoundedDigestWriter::new(MAX_INTERRUPTED_VIEW_COMMITMENT_BYTES);
    let encoded = serde_json::to_writer(
        &mut writer,
        &Commitment {
            schema_version: &view.schema_version,
            workspace_identity: &view.workspace_identity,
            prior_journal_head_identity: &view.prior_journal_head_identity,
            prior_head: &view.prior_head,
            prospective_journal_head_identity: &view.prospective_journal_head_identity,
            prospective_head: &view.prospective_head,
            snapshot_id: &view.snapshot_id,
            snapshot: &view.snapshot,
            target: &view.root_action_request.target,
            action: &view.root_action_request,
        },
    );
    if writer.exceeded {
        return Err(ProductError::Kernel(OrchestrationError::ResourceLimit));
    }
    encoded.map_err(|_| ProductError::AuthorityInvalid)?;
    Ok(format!("sha256:{:x}", writer.hasher.finalize()))
}

struct BoundedDigestWriter {
    hasher: Sha256,
    maximum: usize,
    written: usize,
    exceeded: bool,
}

impl BoundedDigestWriter {
    fn new(maximum: usize) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(INTERRUPTED_VIEW_DOMAIN);
        Self {
            hasher,
            maximum,
            written: 0,
            exceeded: false,
        }
    }
}

impl Write for BoundedDigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.written) {
            self.exceeded = true;
            return Err(io::Error::other(
                "bounded interrupted view commitment exceeded",
            ));
        }
        self.hasher.update(bytes);
        self.written += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn require_recoverable(snapshot: &ProductSnapshot) -> Result<(), ProductError> {
    if !snapshot.recovery.ambiguous_operations.is_empty()
        || snapshot.recovery.pending_integration_id.is_some()
    {
        return Err(ProductError::AmbiguousRecovery);
    }
    if !snapshot.recovery.stale_binding_leases.is_empty() {
        return Err(ProductError::StaleCandidate);
    }
    if !snapshot.recovery.expired_leases.is_empty() {
        return Err(ProductError::LeaseExpired);
    }
    if !snapshot.recovery.orphaned_leases.is_empty() {
        return Err(ProductError::UnknownWorker);
    }
    Ok(())
}
