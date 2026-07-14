fn reconcile_current(
    input: &ChildInput,
    workspace: &ProductWorkspace,
    adapter: &OrchestrationRuntimeAdapter<'_>,
) -> Result<ChildOutput, ProductError> {
    let view = adapter.inspect_current(&current_request(input))?;
    let action = input
        .action
        .as_ref()
        .ok_or(ProductError::AuthorityInvalid)?;
    let permit = input
        .permit
        .as_ref()
        .ok_or(ProductError::AuthorityInvalid)?;
    let lease_id = action
        .target
        .lease_id
        .clone()
        .ok_or(ProductError::UnknownWorker)?;
    let resolution = input
        .resolution
        .clone()
        .ok_or(ProductError::AuthorityInvalid)?;
    let request = ReconcileRequest {
        expected_head: input.expected_head.clone(),
        tick: input.tick,
        live_workers: input.live_workers.clone(),
        lease_id,
        resolution,
        target: action.target.clone(),
    };
    let outcome = adapter.execute_reconcile(&view, action, &authority(), permit, &request)?;
    Ok(ChildOutput {
        status: "reconciled".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(outcome.current_head),
        snapshot: None,
        action: Some(action.clone()),
        permit: None,
        read_unchanged: None,
    })
}

fn inspect_interrupted(
    input: &ChildInput,
    workspace: &ProductWorkspace,
    adapter: &OrchestrationRuntimeAdapter<'_>,
) -> Result<ChildOutput, ProductError> {
    let before = recursive_fingerprint(&input.journal_root);
    let view = adapter.inspect_interrupted(&interrupted_request(input)?)?;
    let action = view.preview().root_action_request.clone();
    let permit = permit_for_action(&authority(), &action, input.tick);
    Ok(ChildOutput {
        status: "inspected-interrupted".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(view.preview().prospective_head.clone()),
        snapshot: Some(view.preview().snapshot.clone()),
        action: Some(action),
        permit: Some(permit),
        read_unchanged: Some(recursive_fingerprint(&input.journal_root) == before),
    })
}

fn recover_interrupted(
    input: &ChildInput,
    workspace: &ProductWorkspace,
    adapter: &OrchestrationRuntimeAdapter<'_>,
) -> Result<ChildOutput, ProductError> {
    let inspection = interrupted_request(input)?;
    let view = adapter.inspect_interrupted(&inspection)?;
    let action = input
        .action
        .as_ref()
        .ok_or(ProductError::AuthorityInvalid)?;
    let permit = input
        .permit
        .as_ref()
        .ok_or(ProductError::AuthorityInvalid)?;
    let request = RuntimeActionRequest::Recover(RecoverRequest {
        expected_prior_head: inspection.expected_prior_head,
        expected_event_id: inspection.expected_event_id,
        recovered_binding: inspection.recovered_binding,
        tick: inspection.tick,
        live_workers: inspection.live_workers,
        target: action.target.clone(),
    });
    let outcome = adapter.execute_action(
        RuntimeActionSource::Interrupted(&view),
        action,
        &authority(),
        permit,
        &request,
    )?;
    let RuntimeActionOutcome::Recover(outcome) = outcome else {
        return Err(ProductError::AuthorityOperationMismatch);
    };
    Ok(ChildOutput {
        status: "recovered".to_owned(),
        error_code: None,
        workspace_identity: Some(outcome.workspace_identity),
        current_head: Some(outcome.recovered_head),
        snapshot: Some(outcome.snapshot),
        action: Some(action.clone()),
        permit: None,
        read_unchanged: None,
    })
}

fn reopen(
    input: &ChildInput,
    workspace: &ProductWorkspace,
    adapter: &OrchestrationRuntimeAdapter<'_>,
) -> Result<ChildOutput, ProductError> {
    let before = recursive_fingerprint(&input.journal_root);
    let view = adapter.inspect_current(&current_request(input))?;
    Ok(ChildOutput {
        status: "reopened".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(view.state().snapshot.journal_head.clone()),
        snapshot: Some(view.state().snapshot.clone()),
        action: view.state().root_action_requests.first().cloned(),
        permit: None,
        read_unchanged: Some(recursive_fingerprint(&input.journal_root) == before),
    })
}
