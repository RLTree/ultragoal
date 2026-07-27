use super::*;

pub(crate) struct RoutineDiagnosisBinding {
    target: PathBuf,
    context: LiveContext,
    manifest: LoadedManifest,
    graph: ImpactGraph,
    plan: RoutinePlan,
    snapshot: crate::routine_work::DirtySnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineCheckpointProjection {
    pub(crate) state: String,
    pub(crate) terminal_outcome: Option<String>,
    pub(crate) event_id: String,
    pub(crate) event_status: String,
    pub(crate) event_transition: String,
    pub(crate) finding_id: Option<String>,
}

impl RoutineDiagnosisBinding {
    pub(crate) fn target(&self) -> &Path {
        &self.target
    }

    pub(crate) fn context(&self) -> &LiveContext {
        &self.context
    }

    pub(crate) fn plan(&self) -> &RoutinePlan {
        &self.plan
    }

    pub(crate) fn manifest(&self) -> &LoadedManifest {
        &self.manifest
    }

    pub(crate) fn graph(&self) -> &ImpactGraph {
        &self.graph
    }

    pub(crate) fn snapshot(&self) -> &crate::routine_work::DirtySnapshot {
        &self.snapshot
    }
}

pub(crate) fn current_diagnosis_binding(
    root: &Path,
    requested_target: Option<&str>,
    read_context: &LiveContext,
) -> Result<Option<RoutineDiagnosisBinding>, ()> {
    let target = source_context::target_root(
        root,
        &source_context::observability_options(requested_target),
    )
    .map_err(|_| ())?;
    match fs::symlink_metadata(target.join(MANIFEST_PATH)) {
        Ok(_) => {}
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound && requested_target.is_none() =>
        {
            return Ok(None);
        }
        Err(_) => return Err(()),
    }
    let discovery = source_context::discovery_context(&target).map_err(|_| ())?;
    let manifest = manifest::load(&discovery, &target).map_err(|_| ())?;
    let graph = manifest.graph().map_err(|_| ())?;
    let context = source_context::execution_context(&target, &manifest).map_err(|_| ())?;
    validate_selected_sources(&context, &manifest).map_err(|_| ())?;
    let snapshot = LocalDirtyTree::capture(&context).map_err(|_| ())?;
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine()).map_err(|_| ())?;
    if read_context.revalidate().is_err() || context.revalidate().is_err() {
        return Err(());
    }
    Ok(Some(RoutineDiagnosisBinding {
        target,
        context,
        manifest,
        graph,
        plan,
        snapshot,
    }))
}

pub(crate) fn current_checkpoint(
    home: &Path,
    binding: &RoutineDiagnosisBinding,
) -> Result<Option<RoutineCheckpointProjection>, ()> {
    let state = match HostState::open_existing(home) {
        Ok(state) => state,
        Err(host::HostFailure::Unavailable) => return Ok(None),
        Err(_) => return Err(()),
    };
    let execution_id = prepare(
        binding.context(),
        binding.manifest(),
        binding.graph(),
        binding.snapshot(),
        binding.plan(),
    )
    .map_err(|_| ())?
    .checkpoint_execution_id()
    .to_owned();
    let checkpoint = state
        .exact_checkpoint(
            host::CheckpointBinding::new(
                binding.target(),
                binding.context().context_id(),
                binding.plan().binding().candidate_id(),
                binding.plan().plan_id(),
                binding.snapshot().snapshot_id(),
                &execution_id,
            ),
            None,
        )
        .map_err(|_| ())?;
    if let Some(checkpoint) = checkpoint.as_ref() {
        state
            .authenticate_checkpoint(binding.target(), checkpoint, false)
            .map_err(|_| ())?;
    }
    if state.verify().is_err() || binding.context().revalidate().is_err() {
        return Err(());
    }
    Ok(checkpoint.map(|checkpoint| RoutineCheckpointProjection {
        state: checkpoint.state().to_owned(),
        terminal_outcome: checkpoint
            .terminal_outcome()
            .map(|outcome| outcome.as_str().to_owned()),
        event_id: checkpoint.event_id().to_owned(),
        event_status: checkpoint.event_status().to_owned(),
        event_transition: checkpoint.event_transition().to_owned(),
        finding_id: checkpoint
            .finding_binding()
            .map(|binding| binding.finding_id.clone()),
    }))
}
