use super::*;

/// Exercises the descriptor-held output preparation and final validation used
/// by mediation without constructing or consuming execution authority.
pub(crate) fn test_probe_output_confinement(
    root: &Path,
    scopes: &[RepoPath],
    budget_bytes: u64,
    after_prepare: impl FnOnce(),
) -> Result<(), RoutineError> {
    let root = RootAnchor::open(root)?;
    let outputs = OutputConfinement::prepare(&root, scopes, budget_bytes)?;
    after_prepare();
    outputs.validate()?;
    outputs.capture_owned_delta().map(|_| ())
}

/// Binds exact read descriptors, permits mutation at the real between-bind
/// boundary, then reopens and validates the production records.
pub(crate) fn test_probe_read_confinement(
    root: &Path,
    sources: &[RepoPath],
    after_bind: impl FnOnce(),
) -> Result<(), RoutineError> {
    let root = RootAnchor::open(root)?;
    let records = ReadConfinement::bind_records(&root, sources)?;
    after_bind();
    let reads = ReadConfinement::open_bound(&root, &records)?;
    reads.validate(&root)
}
