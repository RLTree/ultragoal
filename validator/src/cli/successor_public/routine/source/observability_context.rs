use super::*;

pub(crate) fn current_observability_context(
    root: &Path,
    read_context: &LiveContext,
) -> Result<Option<LiveContext>, ()> {
    let target = fs::canonicalize(root).map_err(|_| ())?;
    match fs::symlink_metadata(target.join(MANIFEST_PATH)) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(()),
    }

    let discovery = source_context::discovery_context(&target).map_err(|_| ())?;
    let manifest = manifest::load(&discovery, &target).map_err(|_| ())?;
    let context = source_context::execution_context(&target, &manifest).map_err(|_| ())?;
    if context.candidate() != read_context.candidate()
        || context.worktree_root() != read_context.worktree_root()
        || read_context.revalidate().is_err()
        || context.revalidate().is_err()
    {
        return Err(());
    }
    Ok(Some(context))
}
