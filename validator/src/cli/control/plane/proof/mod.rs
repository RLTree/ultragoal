use crate::cli::control::plane::types::ControlOperation;
use serde_json::Value;
use std::path::Path;

mod authority;
mod diagnostic;
mod red;
pub(crate) mod transaction;

pub(crate) fn failures(root: &Path, operation: ControlOperation) -> Vec<String> {
    if matches!(
        operation,
        ControlOperation::RegistryProbe | ControlOperation::AppSurfaceProbe
    ) {
        return authority::registry_surface(root);
    }
    let mut out = authority::all(root);
    out.extend(transaction::failures(root, operation));
    out
}

pub(crate) fn failure_value(operation: ControlOperation, failures: &[String]) -> Value {
    diagnostic::failure_value(operation, failures)
}

pub(crate) fn notes(pass: bool, failures: &[String]) -> String {
    diagnostic::notes(pass, failures)
}
