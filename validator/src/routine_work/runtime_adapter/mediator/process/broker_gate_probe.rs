use super::*;
use crate::routine_work::RepoPath;

/// Enters the exact bounded-process implementation with real immutable program,
/// root, read, and output descriptors. It exposes refusal only and cannot mint
/// a grant, broker response, reservation, or successful observation.
pub(crate) fn test_probe_execute_without_root_broker(
    root: &Path,
    immutable_program: &Path,
    output_scopes: &[RepoPath],
    read_sources: &[RepoPath],
) -> Result<(), RoutineError> {
    let root = RootAnchor::open(root)?;
    let program = PinnedExecutable::open_unbound(immutable_program)?;
    let outputs = OutputConfinement::prepare(&root, output_scopes, 1024 * 1024)?;
    let records = ReadConfinement::bind_records(&root, read_sources)?;
    let reads = ReadConfinement::open_bound(&root, &records)?;
    let frame = reads.rust_source_syntax_frame(&root)?;
    let environment = BTreeMap::from([
        ("LANG".to_owned(), "C".to_owned()),
        ("LC_ALL".to_owned(), "C".to_owned()),
        ("PATH".to_owned(), "/usr/bin".to_owned()),
    ]);
    execute(
        &program,
        &root,
        &outputs,
        &reads,
        &[
            "ultragoal".to_owned(),
            "--json".to_owned(),
            "check".to_owned(),
            "routine".to_owned(),
        ],
        &environment,
        frame,
        Duration::from_secs(1),
        1024 * 1024,
        &RoutineCancellation::new(),
        || -> Result<(), RoutineError> { panic!("broker refusal must precede Started") },
    )
    .map(|_| ())
}
