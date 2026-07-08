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
    let untyped_command_enum =
        crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
            "validator/src/command/mod.rs",
            "use std::path::PathBuf;\npub(crate) enum Command { Audit { receipt: PathBuf } }\n",
        );
    assert!(
        untyped_command_enum
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_path")),
        "{untyped_command_enum:?}"
    );

    let command_projection = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/command/mod.rs",
        "use std::path::PathBuf;\npub(crate) const EXECUTION_PROJECTION_ROLE: &str = \"execution_projection_from_typed_cli_authority\";\npub(crate) enum Command { Audit { receipt: PathBuf } }\n",
    );
    assert!(command_projection.is_empty(), "{command_projection:?}");

    let parser = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/cli/performance/mod.rs",
        "use std::path::PathBuf;\npub(crate) struct PerformanceCommand { receipt: Option<PathBuf> }\npub(crate) fn parse(raw: &[String]) -> Result<Option<PerformanceCommand>, String> { Ok(None) }\n",
    );
    assert!(parser.is_empty(), "{parser:?}");
}

#[test]
fn raw_authority_scanner_rejects_parser_that_exports_untyped_paths() {
    let direct_parser = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/argument_parser/mod.rs",
        "use std::path::PathBuf;\npub(crate) fn parse(raw: &[String]) -> PathBuf { PathBuf::from(raw[0].clone()) }\n",
    );
    assert!(
        direct_parser
            .iter()
            .any(|failure| failure.contains("raw_authority=raw_path")),
        "{direct_parser:?}"
    );

    let typed_parser = crate::audit::law::authority_surfaces::raw_authority_failures_for_test(
        "validator/src/argument_parser/authority.rs",
        "use std::path::PathBuf;\npub(super) struct CliRoot { path: PathBuf }\nfn typed_path(raw: &str, product_role: &str) -> Result<PathBuf, String> { Ok(PathBuf::from(raw)) }\n",
    );
    assert!(typed_parser.is_empty(), "{typed_parser:?}");
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
