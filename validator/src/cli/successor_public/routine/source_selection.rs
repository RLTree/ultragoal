use super::*;

const RESULT_SCOPE: &str = "routine-public-production";

pub(crate) fn validate_selected_sources(
    context: &LiveContext,
    manifest: &LoadedManifest,
) -> Result<(), PublicFailure> {
    let inputs = manifest::selected_input_map(context);
    if inputs.get(MANIFEST_PATH)
        != Some(&(manifest.source_sha256.clone(), manifest.source_byte_length))
        || inputs.get(&manifest.catalog.path)
            != Some(&(
                manifest.catalog.sha256.clone(),
                manifest.catalog.byte_length,
            ))
    {
        return Err(PublicFailure::Manifest(
            manifest::ManifestFailure::ConcurrentMutation,
        ));
    }
    Ok(())
}

pub(crate) fn prepare(
    context: &LiveContext,
    manifest: &LoadedManifest,
    graph: &ImpactGraph,
    snapshot: &crate::routine_work::DirtySnapshot,
    plan: &RoutinePlan,
) -> Result<PreparedRoutineExecution, PublicFailure> {
    if plan.checks().is_empty() {
        return prepare_routine_execution(
            context,
            graph,
            snapshot,
            plan,
            RoutineAdapterSpec::new(RESULT_SCOPE, Vec::new()),
        )
        .map_err(PublicFailure::Routine);
    }

    let adoption_nodes = manifest
        .nodes
        .iter()
        .map(|node| AdoptedRoutineNode::new(&node.node_id, node.depends_on.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| PublicFailure::Catalog(error.code()))?;
    let adoption = CatalogAdoption::new(
        &manifest.catalog.sha256,
        manifest.catalog.byte_length,
        graph.graph_id(),
        plan.binding().candidate_id(),
        adoption_nodes,
    )
    .map_err(|error| PublicFailure::Catalog(error.code()))?;
    let catalog = load_production_catalog(
        context.worktree_root(),
        Path::new(&manifest.catalog.path),
        adoption,
    )
    .map_err(|error| PublicFailure::Catalog(error.code()))?;
    let inputs = manifest::selected_input_map(context);
    let selected = plan
        .checks()
        .iter()
        .map(|check| {
            let node = manifest
                .node(check.node_id())
                .ok_or(PublicFailure::Catalog(
                    "routine-public-selected-node-unknown",
                ))?;
            let transitive = node
                .read_sources
                .iter()
                .map(|path| {
                    let (digest, length) = inputs.get(path).ok_or(PublicFailure::Catalog(
                        "routine-public-selected-input-missing",
                    ))?;
                    TransitiveInputExpectation::new(path, digest, *length)
                        .map_err(|error| PublicFailure::Catalog(error.code()))
                })
                .collect::<Result<Vec<_>, _>>()?;
            SelectedRoutineNode::new(
                check.node_id(),
                check.depends_on().iter().cloned(),
                check.input_id(),
                transitive,
            )
            .map_err(|error| PublicFailure::Catalog(error.code()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut observed_tools = BTreeSet::new();
    let runners = plan
        .checks()
        .iter()
        .filter(|check| observed_tools.insert(check.selected_tool().to_owned()))
        .map(|check| {
            runner_observation(
                context,
                check.selected_tool(),
                check.selected_tool_identity(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let request = CatalogSelectionRequest::new(
        catalog.catalog_id(),
        graph.graph_id(),
        plan.binding().candidate_id(),
        plan.plan_id(),
        selected,
        runners,
    )
    .map_err(|error| PublicFailure::Catalog(error.code()))?;
    let bound = catalog
        .bind_selected(request)
        .map_err(|error| PublicFailure::Catalog(error.code()))?;
    if bound.invocations().len() != plan.checks().len() {
        return Err(PublicFailure::Catalog(
            "routine-public-bound-cardinality-invalid",
        ));
    }
    let invocations = bound
        .invocations()
        .iter()
        .map(|invocation| bind_public_invocation(context, manifest, plan, invocation))
        .collect::<Result<Vec<_>, _>>()?;
    prepare_routine_execution(
        context,
        graph,
        snapshot,
        plan,
        RoutineAdapterSpec::new(RESULT_SCOPE, invocations),
    )
    .map_err(PublicFailure::Routine)
}

pub(crate) fn runner_observation(
    context: &LiveContext,
    tool_name: &str,
    tool_identity: &str,
) -> Result<RunnerObservation, PublicFailure> {
    let capability = context
        .capabilities()
        .tool(tool_name)
        .ok_or(PublicFailure::Catalog("routine-public-runner-unobserved"))?;
    let executable = safe_executable(capability, tool_name)?;
    RunnerObservation::new(
        tool_name,
        tool_identity,
        executable,
        format!(
            "sha256:{}",
            capability
                .executable_sha256
                .as_deref()
                .ok_or(PublicFailure::Catalog(
                    "routine-public-runner-digest-missing"
                ))?
        ),
        capability.byte_length.ok_or(PublicFailure::Catalog(
            "routine-public-runner-length-missing",
        ))?,
        capability
            .unix_mode
            .ok_or(PublicFailure::Catalog("routine-public-runner-mode-missing"))?,
    )
    .map_err(|error| PublicFailure::Catalog(error.code()))
}

pub(crate) fn safe_executable(
    capability: &ToolCapability,
    tool_name: &str,
) -> Result<PathBuf, PublicFailure> {
    if tool_name != manifest::ROUTINE_RUNNER {
        return Err(PublicFailure::Catalog(
            "routine-public-runner-not-allowlisted",
        ));
    }
    let expected = std::env::current_exe()
        .and_then(fs::canonicalize)
        .map_err(|_| PublicFailure::Catalog("routine-public-current-runner-unavailable"))?;
    if !capability.available
        || capability.executable.as_deref().map(Path::new) != Some(expected.as_path())
    {
        return Err(PublicFailure::Catalog(
            "routine-public-runner-path-substituted",
        ));
    }
    validate_immutable_routine_program(&expected)
        .map_err(|_| PublicFailure::Catalog("routine-public-current-runner-not-immutable"))?;
    Ok(expected)
}
