use super::*;

pub(crate) fn status_kind(status: ExitStatus) -> ProcessTermination {
    status.code().map_or_else(
        || ProcessTermination::Signaled(status.signal().unwrap_or(0)),
        ProcessTermination::Exited,
    )
}
