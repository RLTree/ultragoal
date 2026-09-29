use super::*;

pub(crate) struct RoutineObservabilityBinding {
    pub(crate) target: PathBuf,
    pub(crate) context: LiveContext,
}

pub(crate) fn current_observability_context(
    root: &Path,
    requested_target: Option<&str>,
    read_context: &LiveContext,
) -> Result<Option<RoutineObservabilityBinding>, ()> {
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
    let context = source_context::execution_context(&target, &manifest).map_err(|_| ())?;
    if read_context.revalidate().is_err() || context.revalidate().is_err() {
        return Err(());
    }
    Ok(Some(RoutineObservabilityBinding { target, context }))
}
