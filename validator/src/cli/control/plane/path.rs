use crate::cli::control::plane::types::ControlOperation;
use std::path::{Component, Path, PathBuf};

pub(crate) fn validate_receipt_path(
    root: &Path,
    path: &Path,
    operation: ControlOperation,
) -> Result<(), String> {
    let actual = relative_receipt_path(root, path)?;
    let expected = PathBuf::from(expected_receipt_path(operation));
    if actual != expected {
        return Err(format!(
            "cli_control_plane_receipt_path_mismatch: operation={} expected={} actual={}",
            operation.id(),
            expected.display(),
            actual.display()
        ));
    }
    Ok(())
}

pub(crate) fn expected_receipt_path(operation: ControlOperation) -> String {
    format!(
        "validation_artifacts/cli/{}",
        expected_receipt_file(operation)
    )
}

fn expected_receipt_file(operation: ControlOperation) -> String {
    match operation {
        ControlOperation::UpdateGoalEligibility => "update-goal-eligibility.json".to_string(),
        ControlOperation::SelfUpdateGoalEligibility => "self-law-receipt.json".to_string(),
        ControlOperation::RegistryProbe => "registry-probe-receipt.json".to_string(),
        ControlOperation::PacketVerify => "packet-verify-receipt.json".to_string(),
        _ => format!("{}-receipt.json", operation.id().replace('_', "-")),
    }
}

fn relative_receipt_path(_root: &Path, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Err(format!(
            "cli_control_plane_receipt_path_absolute_claim_output:{}",
            path.display()
        ))
    } else {
        normalize_relative_path(path)
    }
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf, String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::Normal(part) => out.push(part),
            Component::ParentDir => {
                return Err(format!(
                    "cli_control_plane_receipt_path_parent_escape:{}",
                    path.display()
                ));
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err("cli_control_plane_receipt_path_empty".to_string());
    }
    Ok(out)
}
