use std::path::Path;

pub(super) fn complete(root: &Path) -> Result<(), String> {
    let failures = crate::audit::observability::command_inventory_failures(root);
    Err(failure_summary(root, &failures))
}

pub(super) fn failure_summary(_root: &Path, failures: &[String]) -> String {
    let blocker = failures
        .first()
        .map(String::as_str)
        .unwrap_or("HCT-OBSERVE successor catalog unavailable/not adopted");
    format!(
        "{blocker}; command inventory deauthorized; first_failure={blocker}; control_board_first_incomplete=HCT-OBSERVE"
    )
}
