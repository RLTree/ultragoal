use super::*;

pub(crate) const SOURCE_CONFIG_KEY: &str = "contract_id";
pub(crate) const RESULT_SCOPE: &str = "routine-public-production";

struct CachePublisher<'a> {
    state: &'a HostState,
    binding: CacheBinding,
}

impl RoutineArtifactPublisher for CachePublisher<'_> {
    fn publish(&self, artifacts: &[Vec<u8>]) -> Result<(), crate::routine_work::RoutineError> {
        self.state
            .persist_reuse(self.binding.clone(), artifacts)
            .map_err(|_| {
                crate::routine_work::RoutineError::new(
                    crate::routine_work::RoutineErrorId::ObservationFailed,
                    "routine-public-cache-publication-failed",
                    None,
                )
            })
    }
}

pub(crate) fn execute(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> RuntimeOutcome {
    if let Some(outcome) = behavior_child::execute_if_requested(invocation) {
        return outcome;
    }
    execute_inner(root, invocation, home).unwrap_or_else(outcome::failure)
}

pub(crate) fn execute_inner(
    root: &Path,
    invocation: &ParsedInvocation,
    home: Option<&Path>,
) -> Result<RuntimeOutcome, PublicFailure> {
    let target = target_root(root, invocation)?;
    let discovery = discovery_context(&target)?;
    let manifest = manifest::load(&discovery, &target).map_err(PublicFailure::Manifest)?;
    let graph = manifest
        .graph()
        .map_err(|_| PublicFailure::Manifest(manifest::ManifestFailure::Invalid))?;
    let context = execution_context(&target, &manifest)?;
    validate_selected_sources(&context, &manifest)?;
    let snapshot = LocalDirtyTree::capture(&context).map_err(PublicFailure::Routine)?;
    let plan = plan_routine(&context, &graph, &snapshot, PlanRequest::routine())
        .map_err(PublicFailure::Routine)?;
    let source_id = manifest.source_id();

    if plan.checks().is_empty() {
        let prepared = prepare(&context, &manifest, &graph, &snapshot, &plan)?;
        let result = mediate_prepared_routine_execution_production(
            Path::new("/routine-noop-does-not-open-authority"),
            &context,
            &plan,
            prepared,
            None,
            RoutineCancellation::new(),
            RoutineReuseInput::default(),
        )
        .map_err(PublicFailure::Routine)?;
        return Ok(outcome::mediation(
            &result,
            context.context_id(),
            plan.binding().candidate_id(),
            graph.graph_id(),
            snapshot.snapshot_id(),
            plan.plan_id(),
            &source_id,
            None,
            plan.affected_set().coverage().fallback_tool_count(),
        ));
    }

    let home = home.ok_or(PublicFailure::Host(host::HostFailure::Unavailable))?;
    let state = HostState::open(home, &target).map_err(PublicFailure::Host)?;
    let selected_nodes = plan
        .checks()
        .iter()
        .map(|check| check.node_id().to_owned())
        .collect::<Vec<_>>();
    let provision = state
        .provision_outputs(&target, &selected_nodes)
        .map_err(PublicFailure::Host)?;
    let prepared = match prepare(&context, &manifest, &graph, &snapshot, &plan) {
        Ok(prepared) => prepared,
        Err(error) => {
            provision
                .rollback()
                .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
            return Err(error);
        }
    };
    let request = match &prepared {
        PreparedRoutineExecution::Effect(request) => request,
        PreparedRoutineExecution::NoOp(_) => unreachable!("no-op returned above"),
    };
    let request_id = request.request_id().to_owned();
    let protocol_id = request.protocol_id().to_owned();
    let cache_binding = CacheBinding::new(
        state.target_id().to_owned(),
        source_id.clone(),
        context.context_id(),
        plan.binding().candidate_id(),
        graph.graph_id(),
        snapshot.snapshot_id(),
        plan.plan_id(),
        &protocol_id,
        &request_id,
    );
    let reuse = state
        .read_reuse(&cache_binding)
        .map_err(PublicFailure::Host)?;

    let mediated = (|| {
        let issuer = if reuse.is_some() {
            ProductionRoutineIssuer::open_existing(state.authority_root())
        } else {
            ProductionRoutineIssuer::open(state.authority_root())
        }
        .map_err(PublicFailure::Routine)?;
        let recovery = issuer
            .pending_recovery(&context, &plan, &prepared)
            .map_err(PublicFailure::Routine)?;
        let publisher = CachePublisher {
            state: &state,
            binding: cache_binding,
        };
        issuer
            .mediate_with_publisher(
                &context,
                &plan,
                prepared,
                recovery,
                RoutineCancellation::new(),
                reuse.map_or_else(RoutineReuseInput::default, RoutineReuseInput::new),
                &publisher,
            )
            .map_err(PublicFailure::Routine)
    })();
    let result = match mediated {
        Ok(result) => result,
        Err(error) => {
            provision
                .rollback()
                .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
            return Err(error);
        }
    };

    state
        .verify()
        .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
    if result.status() == RoutineMediatorStatus::CompleteExecution {
        provision.commit();
    } else {
        provision
            .rollback()
            .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
    }
    state
        .verify()
        .map_err(|_| PublicFailure::PersistenceAfterEffect)?;
    Ok(outcome::mediation(
        &result,
        context.context_id(),
        plan.binding().candidate_id(),
        graph.graph_id(),
        snapshot.snapshot_id(),
        plan.plan_id(),
        &source_id,
        Some(&protocol_id),
        plan.affected_set().coverage().fallback_tool_count(),
    ))
}

pub(crate) fn target_root(
    root: &Path,
    invocation: &ParsedInvocation,
) -> Result<PathBuf, PublicFailure> {
    if invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
    {
        return Err(PublicFailure::InvalidInvocation);
    }
    let mut relative = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RelativePath(path)) if relative.is_none() => {
                relative = Some(path.as_str())
            }
            _ => return Err(PublicFailure::InvalidInvocation),
        }
    }
    let canonical_root = fs::canonicalize(root).map_err(|_| PublicFailure::Context)?;
    if canonical_root != root {
        return Err(PublicFailure::Context);
    }
    let requested = relative.map_or_else(|| root.to_path_buf(), |path| root.join(path));
    let metadata = fs::symlink_metadata(&requested).map_err(|_| PublicFailure::Context)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PublicFailure::Context);
    }
    let target = fs::canonicalize(&requested).map_err(|_| PublicFailure::Context)?;
    if !target.starts_with(root) || target.to_str().is_none() {
        return Err(PublicFailure::Context);
    }
    Ok(target)
}

pub(crate) fn discovery_context(target: &Path) -> Result<LiveContext, PublicFailure> {
    LiveContext::build(
        BuildRequest::new(target)
            .expect_worktree_root(target)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            )
            .select_input(target.join(MANIFEST_PATH)),
    )
    .map_err(|_| PublicFailure::Context)
}

pub(crate) fn execution_context(
    target: &Path,
    manifest: &LoadedManifest,
) -> Result<LiveContext, PublicFailure> {
    let mut request = BuildRequest::new(target)
        .expect_worktree_root(target)
        .with_effect(EffectClass::Read)
        .bind_non_secret_configuration(
            ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
            ADOPTED_HANDOFF_MANIFEST_SHA256,
        )
        .bind_non_secret_configuration(SOURCE_CONFIG_KEY, manifest.source_id());
    for path in manifest.selected_paths() {
        request = request.select_input(target.join(path));
    }
    for tool in manifest.tool_names() {
        request = request.probe_tool(tool);
    }
    LiveContext::build(request).map_err(|_| PublicFailure::Context)
}
