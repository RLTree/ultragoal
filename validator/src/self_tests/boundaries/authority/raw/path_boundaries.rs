#[test]
fn raw_authority_scanner_rejects_unclassified_path_authority() {
    let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/domain/claim_core.rs",
        "use std::path::PathBuf;\npub(crate) fn decide(raw_path: PathBuf) -> bool { raw_path.is_absolute() }\n",
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_path")),
        "{failures:?}"
    );
}

#[test]
fn raw_authority_scanner_allows_typed_cli_command_path_boundaries() {
    let command_enum = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/command/mod.rs",
        "use std::path::PathBuf;\npub(crate) enum Command { Audit { receipt: PathBuf } }\n",
    );
    assert!(command_enum.is_empty(), "{command_enum:?}");

    let parser = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/performance/mod.rs",
        "use std::path::PathBuf;\npub(crate) struct PerformanceCommand { receipt: Option<PathBuf> }\npub(crate) fn parse(raw: &[String]) -> Result<Option<PerformanceCommand>, String> { Ok(None) }\n",
    );
    assert!(parser.is_empty(), "{parser:?}");
}

#[test]
fn raw_authority_scanner_allows_typed_law_check_path_boundaries() {
    let failures = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/audit/review_history.rs",
        "use std::collections::BTreeMap;\nuse std::path::Path;\npub fn check(root: &Path, failures: &mut BTreeMap<String, Vec<String>>) { failures.entry(\"validator-execution-provenance\".to_string()).or_default().push(format!(\"review loop record missing\")); }\n",
    );
    assert!(failures.is_empty(), "{failures:?}");

    let orchestrator = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/audit/observability/mod.rs",
        "use std::path::Path;\npub(crate) fn package_failures(root: &Path) -> Vec<String> { let mut out = Vec::new(); files::check(root, &mut out); out }\n",
    );
    assert!(orchestrator.is_empty(), "{orchestrator:?}");

    let optional_failure = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/audit/namespace/source/identifiers.rs",
        "use std::path::Path;\nfn identifier_failure(root: &Path) -> Option<String> { Some(format!(\"{}\", root.display())) }\n",
    );
    assert!(optional_failure.is_empty(), "{optional_failure:?}");

    let vector_failure = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/audit/observability/files.rs",
        "use std::path::Path;\nfn failures(root: &Path) -> Vec<String> { vec![format!(\"{}\", root.display())] }\n",
    );
    assert!(vector_failure.is_empty(), "{vector_failure:?}");
}
