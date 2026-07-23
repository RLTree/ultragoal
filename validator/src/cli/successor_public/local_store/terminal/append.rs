use super::super::{LocalStoreFailure, RUNTIME_SOURCE_ID, store_path};
use super::observations::{RoutineTerminalEvent, terminal_event_id};
use crate::context::LiveContext;
use crate::observability::{EventStore, SemanticEvent};
use crate::routine_work::require_runtime_store_ignored;
use std::fs;
use std::path::Path;

pub(in super::super::super) fn append_routine_terminal(
    root: &Path,
    context: &LiveContext,
    binding: &crate::routine_work::RoutineBinding,
    terminal: RoutineTerminalEvent<'_>,
) -> Result<bool, LocalStoreFailure> {
    append_routine_terminal_with_hook(root, context, binding, terminal, None)
}

pub(super) fn append_routine_terminal_with_hook(
    root: &Path,
    context: &LiveContext,
    binding: &crate::routine_work::RoutineBinding,
    terminal: RoutineTerminalEvent<'_>,
    mut before_append_admission: Option<&mut dyn FnMut()>,
) -> Result<bool, LocalStoreFailure> {
    require_runtime_store_ignored(binding, RUNTIME_SOURCE_ID)
        .map_err(|error| LocalStoreFailure::append(error.cause()))?;
    if let Some(hook) = before_append_admission.as_mut() {
        hook();
    }
    require_runtime_store_ignored(binding, RUNTIME_SOURCE_ID)
        .map_err(|error| LocalStoreFailure::append(error.cause()))?;
    ensure_store_parent(root)?;
    let path = store_path(root, context, RUNTIME_SOURCE_ID)?;
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
        (
            "routine_terminal_outcome",
            terminal.terminal_outcome.as_str(),
        ),
    ] {
        event
            .add_public_attribute(key, value)
            .map_err(|error| LocalStoreFailure::append(&error))?;
    }
    if let Some(binding) = terminal.finding_binding {
        event
            .add_finding_ref(&binding.finding_id)
            .and_then(|_| event.add_repair_ref(&binding.repair_id))
            .map_err(|error| LocalStoreFailure::append(&error))?;
    }
    let store = EventStore::for_context(&path, context, RUNTIME_SOURCE_ID)
        .map_err(|error| LocalStoreFailure::append(&error))?;
    require_runtime_store_ignored(binding, RUNTIME_SOURCE_ID)
        .map_err(|error| LocalStoreFailure::append(error.cause()))?;
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
