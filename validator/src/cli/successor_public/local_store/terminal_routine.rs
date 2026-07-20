use super::*;
use sha2::{Digest, Sha256};

/// A terminal routine observation. It is diagnostic-only: it names neither a
/// finding nor a claim and cannot change product-state authority.
pub(crate) struct RoutineTerminalEvent<'a> {
    pub(crate) event_id: &'a str,
    pub(crate) continuation_id: &'a str,
    pub(crate) terminal_ledger_head: &'a str,
    pub(crate) observed_at_unix_ms: u64,
    pub(crate) sequence: u64,
    pub(crate) parent_event_id: Option<&'a str>,
    pub(crate) status: &'a str,
    pub(crate) transition: &'a str,
    pub(crate) findings: &'a [crate::state::Finding],
}

pub(crate) fn append_routine_terminal(
    root: &Path,
    context: &LiveContext,
    terminal: RoutineTerminalEvent<'_>,
) -> Result<bool, LocalStoreFailure> {
    ensure_store_parent(root)?;
    let path = store_path(root);
    let store_absent_before = store_absent(&path)?;
    if terminal.event_id
        != terminal_event_id(terminal.continuation_id, terminal.terminal_ledger_head)
    {
        return Err(LocalStoreFailure::append(
            "observe-routine-terminal-event-id-mismatch",
        ));
    }
    let mut event = SemanticEvent::for_context(
        context,
        RUNTIME_SOURCE_ID,
        terminal.event_id,
        terminal.observed_at_unix_ms,
        terminal.sequence,
        "check.routine.terminal",
        terminal.status,
    )
    .map_err(|error| LocalStoreFailure::append(&error))?;
    if let Some(parent_event_id) = terminal.parent_event_id {
        event
            .set_parent(parent_event_id)
            .map_err(|error| LocalStoreFailure::append(&error))?;
    }
    for (key, value) in [
        ("continuation_id", terminal.continuation_id),
        ("terminal_ledger_head", terminal.terminal_ledger_head),
        ("routine_transition", terminal.transition),
    ] {
        event
            .add_public_attribute(key, value)
            .map_err(|error| LocalStoreFailure::append(&error))?;
    }
    for finding in terminal.findings {
        event
            .add_finding(finding)
            .map_err(|error| LocalStoreFailure::append(&error))?;
    }
    let store = EventStore::for_context(&path, context, RUNTIME_SOURCE_ID)
        .map_err(|error| LocalStoreFailure::append(&error))?;
    let appended = store
        .append(&event)
        .map_err(|error| LocalStoreFailure::append(&error))?;
    if appended && store_absent_before {
        fs::File::open(path.parent().expect("store path has parent"))
            .and_then(|parent| parent.sync_all())
            .map_err(|_| LocalStoreFailure::append("observe-store-parent-sync-failed"))?;
    }
    Ok(appended)
}

fn store_absent(path: &Path) -> Result<bool, LocalStoreFailure> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(false),
        Ok(_) | Err(_) => Err(LocalStoreFailure::open_boundary()),
    }
}

pub(crate) fn terminal_event_id(continuation_id: &str, terminal_ledger_head: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-terminal-event-v1\0");
    digest.update(continuation_id.as_bytes());
    digest.update(b"\0");
    digest.update(terminal_ledger_head.as_bytes());
    format!("routine-terminal-{:x}", digest.finalize())
}

pub(crate) fn routine_observations_from_events(
    events: &[SemanticEvent],
) -> Vec<crate::state::RoutineFindingObservation> {
    events
        .iter()
        .filter_map(|event| {
            if event.operation() != "check.routine.terminal" {
                return None;
            }
            let attributes = event.public_attributes();
            let transition = match attributes.get("routine_transition")?.as_str() {
                "interrupted" => crate::state::RoutineObservationTransition::Interrupted,
                "recovered" => crate::state::RoutineObservationTransition::Recovered,
                "reused" => crate::state::RoutineObservationTransition::Reused,
                "executed" => crate::state::RoutineObservationTransition::Executed,
                "failed" => crate::state::RoutineObservationTransition::Failed,
                "cancelled" => crate::state::RoutineObservationTransition::Cancelled,
                _ => return None,
            };
            Some(crate::state::RoutineFindingObservation {
                event_id: event.event_id().to_owned(),
                continuation_id: attributes.get("continuation_id")?.to_owned(),
                terminal_ledger_head: attributes.get("terminal_ledger_head")?.to_owned(),
                transition,
                outcome: event.outcome().to_owned(),
            })
        })
        .collect()
}

fn ensure_store_parent(root: &Path) -> Result<(), LocalStoreFailure> {
    let canonical_root = fs::canonicalize(root).map_err(|_| LocalStoreFailure::open_boundary())?;
    if canonical_root != root {
        return Err(LocalStoreFailure::open_boundary());
    }
    let mut current = root.to_path_buf();
    for component in ["validation_artifacts", "observability", "spool"] {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => return Err(LocalStoreFailure::open_boundary()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|_| LocalStoreFailure::open_boundary())?;
                let metadata = fs::symlink_metadata(&current)
                    .map_err(|_| LocalStoreFailure::open_boundary())?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(LocalStoreFailure::open_boundary());
                }
            }
            Err(_) => return Err(LocalStoreFailure::open_boundary()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::terminal_event_id;

    #[test]
    fn terminal_event_id_is_bound_only_to_continuation_and_terminal_head() {
        assert_eq!(
            terminal_event_id("routine-cont-a", "head-a"),
            terminal_event_id("routine-cont-a", "head-a")
        );
        assert_ne!(
            terminal_event_id("routine-cont-a", "head-a"),
            terminal_event_id("routine-cont-b", "head-a")
        );
        assert_ne!(
            terminal_event_id("routine-cont-a", "head-a"),
            terminal_event_id("routine-cont-a", "head-b")
        );
    }
}
