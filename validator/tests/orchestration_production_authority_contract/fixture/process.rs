pub fn execute_resume(
    journal_root: &Path,
    action: &RootActionRequest,
    permit: &RootPermit,
    authority: &ProductionRootAuthority,
    tick: u64,
) -> Result<(), ultragoal::orchestration::product::ProductError> {
    let context = context();
    let workspace = ProductWorkspace::open(journal_root)?;
    let adapter = OrchestrationRuntimeAdapter::new(&context, &workspace)?;
    let view = adapter.inspect_current(&OrchestrationStateRequest {
        expected_head: action.expected_head.clone(),
        tick,
        live_workers: BTreeSet::new(),
    })?;
    adapter.execute_production_action(
        authority,
        RuntimeActionSource::Current(&view),
        action,
        permit,
        &RuntimeActionRequest::Resume(ResumeRequest {
            expected_head: action.expected_head.clone(),
            tick,
            live_workers: BTreeSet::new(),
            target: action.target.clone(),
        }),
    )?;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChildInput {
    pub authority_root: PathBuf,
    pub barrier_root: PathBuf,
}
