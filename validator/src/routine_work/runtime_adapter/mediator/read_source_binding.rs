use super::*;

pub(crate) fn bind_read_sources(
    root: &Path,
    sources: &[RepoPath],
) -> Result<Vec<RoutineReadSource>, RoutineError> {
    if sources.is_empty() {
        return Ok(Vec::new());
    }
    let root = RootAnchor::open(root)?;
    ReadConfinement::bind_records(&root, sources)
}

pub(crate) fn validate_read_sources(
    root: &Path,
    sources: &[RoutineReadSource],
) -> Result<(), RoutineError> {
    if sources.is_empty() {
        return Ok(());
    }
    let root = RootAnchor::open(root)?;
    ReadConfinement::open_bound(&root, sources)?.validate(&root)
}
