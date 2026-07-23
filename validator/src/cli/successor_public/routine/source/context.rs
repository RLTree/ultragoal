use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RoutineInvocationOptions {
    target: Option<String>,
    interruption: Option<ReservationInterruption>,
    continuation: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReservationInterruption {
    AfterReservation,
}

impl RoutineInvocationOptions {
    pub(crate) fn interruption(&self) -> Option<ReservationInterruption> {
        self.interruption
    }

    pub(crate) fn continuation(&self) -> Option<&str> {
        self.continuation.as_deref()
    }
}

pub(super) fn observability_options(target: Option<&str>) -> RoutineInvocationOptions {
    RoutineInvocationOptions {
        target: target.map(str::to_owned),
        interruption: None,
        continuation: None,
    }
}

pub(crate) fn options(
    invocation: &ParsedInvocation,
) -> Result<RoutineInvocationOptions, PublicFailure> {
    if invocation.command != SuccessorCommand::Check(CheckProfile::Routine)
        || invocation.effect != EffectClass::WorkspaceWrite
    {
        return Err(PublicFailure::InvalidInvocation);
    }
    let mut target = None;
    let mut interruption = None;
    let mut continuation = None;
    for argument in &invocation.arguments {
        match (&argument.name, &argument.value) {
            (OptionName::Target, ParsedValue::RepositoryTarget(path)) if target.is_none() => {
                target = Some(path.as_str().to_owned());
            }
            (OptionName::InterruptAfter, ParsedValue::Identifier(value))
                if interruption.is_none() && value == "reservation" =>
            {
                interruption = Some(ReservationInterruption::AfterReservation);
            }
            (OptionName::Continuation, ParsedValue::Identifier(value))
                if continuation.is_none()
                    && value.starts_with("routine-cont-")
                    && value.len() > "routine-cont-".len() =>
            {
                continuation = Some(value.to_owned());
            }
            _ => return Err(PublicFailure::InvalidInvocation),
        }
    }
    if interruption.is_some() && continuation.is_some() {
        return Err(PublicFailure::InvalidInvocation);
    }
    Ok(RoutineInvocationOptions {
        target,
        interruption,
        continuation,
    })
}

pub(crate) fn target_root(
    root: &Path,
    options: &RoutineInvocationOptions,
) -> Result<PathBuf, PublicFailure> {
    let canonical_root = fs::canonicalize(root).map_err(|_| PublicFailure::Context)?;
    if root != Path::new(".") && canonical_root != root {
        return Err(PublicFailure::Context);
    }
    let requested = options
        .target
        .as_deref()
        .map_or_else(|| canonical_root.clone(), |path| canonical_root.join(path));
    let metadata = fs::symlink_metadata(&requested).map_err(|_| PublicFailure::Context)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(PublicFailure::Context);
    }
    let target = fs::canonicalize(&requested).map_err(|_| PublicFailure::Context)?;
    if !target.starts_with(&canonical_root) || target.to_str().is_none() {
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
        request = if tool == manifest::ROUTINE_RUNNER {
            request.probe_current_executable(manifest::ROUTINE_RUNNER)
        } else {
            request.probe_tool(tool)
        };
    }
    LiveContext::build(request).map_err(|_| PublicFailure::Context)
}
