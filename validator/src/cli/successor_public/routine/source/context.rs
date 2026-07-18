use super::*;

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
