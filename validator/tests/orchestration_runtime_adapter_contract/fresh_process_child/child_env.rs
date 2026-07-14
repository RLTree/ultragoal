pub const CHILD_ENV: &str = "HUL_ORCHESTRATION_RUNTIME_ADAPTER_INPUT";
pub const CHILD_TEST: &str =
    "orchestration_runtime_adapter_contract::fresh_process_child::fresh_process_child";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildInput {
    pub mode: String,
    pub journal_root: PathBuf,
    pub output: PathBuf,
    pub expected_head: JournalHead,
    pub expected_event_id: Option<String>,
    pub recovered_binding: Option<Binding>,
    pub resolution: Option<EffectResolution>,
    pub tick: u64,
    pub live_workers: BTreeSet<String>,
    pub action: Option<RootActionRequest>,
    pub permit: Option<RootPermit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildOutput {
    pub status: String,
    pub error_code: Option<String>,
    pub workspace_identity: Option<String>,
    pub current_head: Option<JournalHead>,
    pub snapshot: Option<ProductSnapshot>,
    pub action: Option<RootActionRequest>,
    pub permit: Option<RootPermit>,
    pub read_unchanged: Option<bool>,
}

#[test]
fn fresh_process_child() {
    let Some(input_path) = std::env::var_os(CHILD_ENV) else {
        return;
    };
    let input: ChildInput =
        serde_json::from_slice(&fs::read(PathBuf::from(input_path)).unwrap()).unwrap();
    let output = match execute(&input) {
        Ok(output) => output,
        Err(error) => ChildOutput {
            status: "refused".to_owned(),
            error_code: Some(error.code().to_owned()),
            workspace_identity: None,
            current_head: None,
            snapshot: None,
            action: None,
            permit: None,
            read_unchanged: None,
        },
    };
    fs::write(&input.output, serde_json::to_vec(&output).unwrap()).unwrap();
}

fn execute(input: &ChildInput) -> Result<ChildOutput, ProductError> {
    let workspace = ProductWorkspace::open(&input.journal_root)?;
    let context = context();
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace)?;
    match input.mode.as_str() {
        "inspect-current" => inspect_current(input, &workspace, &adapter),
        "resume" => resume_current(input, &workspace, &adapter),
        "reconcile" => reconcile_current(input, &workspace, &adapter),
        "inspect-interrupted" => inspect_interrupted(input, &workspace, &adapter),
        "recover" => recover_interrupted(input, &workspace, &adapter),
        "reopen" => reopen(input, &workspace, &adapter),
        _ => Err(ProductError::AuthorityOperationMismatch),
    }
}

fn current_request(input: &ChildInput) -> OrchestrationStateRequest {
    OrchestrationStateRequest {
        expected_head: input.expected_head.clone(),
        tick: input.tick,
        live_workers: input.live_workers.clone(),
    }
}

fn interrupted_request(input: &ChildInput) -> Result<InterruptedRecoveryRequest, ProductError> {
    Ok(InterruptedRecoveryRequest {
        expected_prior_head: input.expected_head.clone(),
        expected_event_id: input
            .expected_event_id
            .clone()
            .ok_or(ProductError::UnknownOperation)?,
        recovered_binding: input
            .recovered_binding
            .clone()
            .ok_or(ProductError::StaleCandidate)?,
        tick: input.tick,
        live_workers: input.live_workers.clone(),
    })
}

fn inspect_current(
    input: &ChildInput,
    workspace: &ProductWorkspace,
    adapter: &OrchestrationRuntimeAdapter<'_>,
) -> Result<ChildOutput, ProductError> {
    let before = recursive_fingerprint(&input.journal_root);
    let view = adapter.inspect_current(&current_request(input))?;
    for projection in [
        CommandProjection::Inspect,
        CommandProjection::Next,
        CommandProjection::Diagnose(None),
    ] {
        let _ = adapter.project_current(&view, &projection)?;
    }
    let action = view.state().root_action_requests.first().cloned();
    let permit = match action.as_ref().map(|action| action.operation) {
        Some(RootOperation::Resume) => action
            .as_ref()
            .map(|action| permit_for_action(&authority(), action, input.tick)),
        Some(RootOperation::Reconcile) => Some(permit_for_reconciliation(
            &authority(),
            action.as_ref().unwrap(),
            input.tick,
            input
                .resolution
                .as_ref()
                .ok_or(ProductError::AuthorityInvalid)?,
        )),
        Some(RootOperation::Recover) => return Err(ProductError::AuthorityOperationMismatch),
        None => None,
    };
    Ok(ChildOutput {
        status: "inspected-current".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(view.state().snapshot.journal_head.clone()),
        snapshot: Some(view.state().snapshot.clone()),
        action,
        permit,
        read_unchanged: Some(recursive_fingerprint(&input.journal_root) == before),
    })
}

fn resume_current(
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
    let request = RuntimeActionRequest::Resume(ResumeRequest {
        expected_head: input.expected_head.clone(),
        tick: input.tick,
        live_workers: input.live_workers.clone(),
        target: action.target.clone(),
    });
    let outcome = adapter.execute_action(
        RuntimeActionSource::Current(&view),
        action,
        &authority(),
        permit,
        &request,
    )?;
    let RuntimeActionOutcome::Resume(outcome) = outcome else {
        return Err(ProductError::AuthorityOperationMismatch);
    };
    Ok(ChildOutput {
        status: "resumed".to_owned(),
        error_code: None,
        workspace_identity: Some(workspace.identity().to_owned()),
        current_head: Some(outcome.current_head),
        snapshot: Some(outcome.snapshot),
        action: Some(action.clone()),
        permit: None,
        read_unchanged: None,
    })
}
