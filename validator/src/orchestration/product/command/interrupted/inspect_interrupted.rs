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
    let action = RootActionRequest::new(RootActionRequestDefinition {
        operation: RootOperation::Recover,
        reason: RootActionReason::InterruptedPublication,
        authority_binding: context.binding().clone(),
        workspace_identity: workspace.identity().to_owned(),
        expected_head: request.expected_prior_head.clone(),
        snapshot_id: snapshot_id.clone(),
        target,
    })?;
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
