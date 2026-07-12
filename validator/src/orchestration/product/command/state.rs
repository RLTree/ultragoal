use super::super::{
    PermitTarget, ProductContext, ProductError, ProductSnapshot, ProductWorkspace, QueryRequest,
    RootOperation, journal_head_identity, query,
};
use super::action::{RootActionReason, RootActionRequest};
use super::finding::{OrchestrationFinding, action_matches_finding, build_findings};
use crate::orchestration::{EventKind, FileJournal, OrchestrationError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};

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

pub fn next(view: &OrchestrationStateView) -> OrchestrationNext {
    if view.require_authority().is_err() {
        return OrchestrationNext {
            schema_version: "OrchestrationNext-v1".to_owned(),
            state_id: UNTRUSTED_STATE_ID.to_owned(),
            disposition: NextDisposition::NoLegalRoute,
            ready_node: None,
            finding_id: None,
            root_action_request: None,
        };
    }
    if let Some(action) = view.root_action_requests.first() {
        let finding_id = view
            .findings
            .iter()
            .find(|finding| action_matches_finding(action, finding))
            .map(|finding| finding.finding_id.clone());
        return OrchestrationNext {
            schema_version: "OrchestrationNext-v1".to_owned(),
            state_id: view.state_id.clone(),
            disposition: NextDisposition::RootAuthorityRequired,
            ready_node: None,
            finding_id,
            root_action_request: Some(action.clone()),
        };
    }
    if let Some(finding) = view.findings.first() {
        return OrchestrationNext {
            schema_version: "OrchestrationNext-v1".to_owned(),
            state_id: view.state_id.clone(),
            disposition: NextDisposition::NoLegalRoute,
            ready_node: None,
            finding_id: Some(finding.finding_id.clone()),
            root_action_request: None,
        };
    }
    let ready_node = view.snapshot.plan.ready.first().cloned();
    OrchestrationNext {
        schema_version: "OrchestrationNext-v1".to_owned(),
        state_id: view.state_id.clone(),
        disposition: if ready_node.is_some() {
            NextDisposition::ContinueDependencyClosedWork
        } else {
            NextDisposition::NoAction
        },
        ready_node,
        finding_id: None,
        root_action_request: None,
    }
}

fn actions(
    context: &ProductContext,
    workspace: &ProductWorkspace,
    snapshot: &ProductSnapshot,
    snapshot_id: &str,
    pending: &BTreeMap<String, String>,
) -> Result<Vec<RootActionRequest>, ProductError> {
    let mut actions = Vec::new();
    for operation_id in &snapshot.recovery.ambiguous_operations {
        let lease_id = pending
            .get(operation_id)
            .ok_or(ProductError::UnknownOperation)?;
        let target = target_for(snapshot, Some(lease_id.clone()), Some(operation_id.clone()));
        actions.push(RootActionRequest::new(
            RootOperation::Reconcile,
            RootActionReason::EffectOutcomeAmbiguous,
            context.binding().clone(),
            workspace.identity().to_owned(),
            snapshot.journal_head.clone(),
            snapshot_id.to_owned(),
            target,
        )?);
    }
    let blocked_resume = !snapshot.recovery.ambiguous_operations.is_empty()
        || snapshot.recovery.pending_integration_id.is_some()
        || !snapshot.recovery.stale_binding_leases.is_empty()
        || !snapshot.recovery.expired_leases.is_empty()
        || !snapshot.recovery.orphaned_leases.is_empty();
    if snapshot.recovery.interrupted_root && !blocked_resume {
        let lease_id = snapshot.commitments.keys().next().cloned();
        let target = target_for(snapshot, lease_id, None);
        actions.push(RootActionRequest::new(
            RootOperation::Resume,
            RootActionReason::RootInterrupted,
            context.binding().clone(),
            workspace.identity().to_owned(),
            snapshot.journal_head.clone(),
            snapshot_id.to_owned(),
            target,
        )?);
    }
    actions.sort_by_key(|action| (action_priority(action.operation), action.action_id.clone()));
    Ok(actions)
}

fn target_for(
    snapshot: &ProductSnapshot,
    lease_id: Option<String>,
    operation_id: Option<String>,
) -> PermitTarget {
    let result_commitment_id = lease_id.as_ref().and_then(|lease_id| {
        snapshot
            .commitments
            .get(lease_id)
            .map(|commitment| commitment.result_commitment_id.clone())
    });
    PermitTarget {
        lease_id,
        result_commitment_id,
        operation_id,
        recovered_binding: None,
    }
}

fn pending_effect_leases(
    workspace: &ProductWorkspace,
    expected_head: &crate::orchestration::JournalHead,
) -> Result<BTreeMap<String, String>, ProductError> {
    let journal = FileJournal::open(workspace.root()).map_err(ProductError::from)?;
    let snapshot = journal.inspect().map_err(ProductError::from)?;
    if &snapshot.head != expected_head {
        return Err(ProductError::ConcurrentUpdate);
    }
    let mut pending = BTreeMap::new();
    for event in snapshot.log.events() {
        match &event.event {
            EventKind::EffectIntent { lease_id, request } => {
                if pending
                    .insert(request.operation_id.clone(), lease_id.clone())
                    .is_some()
                {
                    return Err(ProductError::AmbiguousRecovery);
                }
            }
            EventKind::EffectApplied { receipt, .. } => {
                pending.remove(&receipt.operation_id);
            }
            EventKind::EffectReconciled { resolution, .. } => {
                pending.remove(&resolution.operation_id);
            }
            _ => {}
        }
    }
    workspace.verify()?;
    Ok(pending)
}

fn state_id(view: &OrchestrationStateView) -> Result<String, ProductError> {
    let mut writer = BoundedDigestWriter::new(MAX_STATE_COMMITMENT_BYTES);
    let encoded = serde_json::to_writer(
        &mut writer,
        &(
            &view.schema_version,
            &view.workspace_identity,
            &view.journal_head_identity,
            &view.snapshot_id,
            &view.snapshot,
            &view.findings,
            &view.root_action_requests,
        ),
    );
    if writer.exceeded {
        return Err(ProductError::Kernel(OrchestrationError::ResourceLimit));
    }
    encoded.map_err(|_| ProductError::StaleCandidate)?;
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
        Self {
            hasher: Sha256::new(),
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
            return Err(io::Error::other("bounded state commitment exceeded"));
        }
        self.hasher.update(bytes);
        self.written += bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn action_priority(operation: RootOperation) -> u8 {
    match operation {
        RootOperation::Reconcile => 0,
        RootOperation::Recover => 1,
        RootOperation::Resume => 2,
    }
}
