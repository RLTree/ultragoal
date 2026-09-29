use crate::context::LiveContext;
use crate::distribution::{CandidateCliPayload, ConfinedRoot, ReadOnlyWorkspace, ScopedFile};

const CLI_LIMIT: usize = 32 * 1024 * 1024;
const CLI_PATH: &str = "target/ultragoal/release/ultragoal";

pub(super) fn allowed(path: &str) -> bool {
    path == CLI_PATH
}

pub(super) fn from_workspace(
    context: &LiveContext,
    path: &str,
    candidate_id: &str,
) -> Option<CandidateCliPayload> {
    let root = ConfinedRoot::open_workspace(context).ok()?;
    let bytes = ScopedFile::new(root, path)
        .ok()?
        .inspect(CLI_LIMIT)
        .ok()??;
    CandidateCliPayload::for_candidate(candidate_id, bytes).ok()
}

pub(super) fn from_read_only(
    context: &LiveContext,
    path: &str,
    candidate_id: &str,
) -> Option<CandidateCliPayload> {
    let workspace = ReadOnlyWorkspace::open(context).ok()?;
    let bytes = workspace.inspect_file(path, CLI_LIMIT).ok()??;
    CandidateCliPayload::for_candidate(candidate_id, bytes).ok()
}
