use crate::context::{ReadSession, query_git};
use crate::inventory::types::InventoryError;
use std::path::Path;

const MAX_FRONTIER_GIT_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

pub(super) fn text(
    reads: &ReadSession,
    root: &Path,
    arguments: &[&str],
    operation: &str,
) -> Result<String, InventoryError> {
    let root = root.to_str().ok_or_else(|| invalid(operation))?;
    let arguments = ["-C", root]
        .into_iter()
        .chain(arguments.iter().copied())
        .collect::<Vec<_>>();
    let bytes = query_git(reads, &arguments).map_err(|_| invalid(operation))?;
    if bytes.len() > MAX_FRONTIER_GIT_OUTPUT_BYTES {
        return Err(invalid(operation));
    }
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|_| invalid(operation))
}

fn invalid(operation: &str) -> InventoryError {
    InventoryError::InvalidRegistry(format!("cannot inspect {operation}"))
}
