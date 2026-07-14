use serde_json::Value;
use std::path::Path;

pub(super) fn receipt_failures(
    root: &Path,
    value: &Value,
    expected_candidate: &str,
    operation: crate::cli::control::plane::operation::ControlOperation,
) -> Vec<String> {
    let store = crate::schema_catalog::load(root);
    let mut out = crate::schema_catalog::schema_errors(
        &store,
        crate::cli::control::plane::surface::SCHEMA_FILE,
        value,
    )
    .into_iter()
    .map(|failure| format!("package_surface_audit_schema:{failure}"))
    .collect::<Vec<_>>();
    out.extend(
        crate::cli::control::plane::surface::same_candidate_pass_or_fail_closed_failures(
            value,
            expected_candidate,
            operation,
        ),
    );
    out
}

#[cfg(test)]
mod tests {
    use crate::cli::control::plane::operation::ControlOperation;
    use serde_json::json;

    #[test]
    fn receipt_failures_prefix_schema_errors() {
        let root = crate::self_tests::boundaries::workspace_fixtures::repo_root();
        let candidate = crate::self_tests::boundaries::workspace_fixtures::sha('a');
        let failures = super::receipt_failures(
            &root,
            &json!({"schema":"wrong"}),
            &candidate,
            ControlOperation::InstallAudit,
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.starts_with("package_surface_audit_schema:")),
            "{failures:?}"
        );
    }
}
