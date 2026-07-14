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
        actions.push(RootActionRequest::new(RootActionRequestDefinition {
            operation: RootOperation::Reconcile,
            reason: RootActionReason::EffectOutcomeAmbiguous,
            authority_binding: context.binding().clone(),
            workspace_identity: workspace.identity().to_owned(),
            expected_head: snapshot.journal_head.clone(),
            snapshot_id: snapshot_id.to_owned(),
            target,
        })?);
    }
    let blocked_resume = !snapshot.recovery.ambiguous_operations.is_empty()
        || snapshot.recovery.pending_integration_id.is_some()
        || !snapshot.recovery.stale_binding_leases.is_empty()
        || !snapshot.recovery.expired_leases.is_empty()
        || !snapshot.recovery.orphaned_leases.is_empty();
    if snapshot.recovery.interrupted_root && !blocked_resume {
        let lease_id = snapshot.commitments.keys().next().cloned();
        let target = target_for(snapshot, lease_id, None);
        actions.push(RootActionRequest::new(RootActionRequestDefinition {
            operation: RootOperation::Resume,
            reason: RootActionReason::RootInterrupted,
            authority_binding: context.binding().clone(),
            workspace_identity: workspace.identity().to_owned(),
            expected_head: snapshot.journal_head.clone(),
            snapshot_id: snapshot_id.to_owned(),
            target,
        })?);
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
