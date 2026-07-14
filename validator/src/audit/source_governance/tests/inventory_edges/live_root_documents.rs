use super::SourceRoot;
use crate::audit::source_governance::scope::SourceClass;

#[test]
fn live_root_documents_are_required_typed_and_report_is_not_active_authority() {
    let root = SourceRoot::new("live-root-documents");
    root.write("REPORT.md", "# Stale predecessor proposal\n");
    let inventory = super::super::super::inventory::capture_once_for_test(root.path())
        .expect("governed inventory");
    for relative in [
        "README.md",
        "DESIGN.md",
        "FRONTEND.md",
        "RELIABILITY.md",
        "PRODUCT_SENSE.md",
        "PRODUCT_FITNESS.md",
        "QUALITY_SCORE.md",
    ] {
        assert!(inventory.sources.iter().any(|source| {
            source.relative == relative && source.class == SourceClass::LiveRootDocument
        }));
    }
    assert!(
        inventory
            .sources
            .iter()
            .all(|source| source.relative != "REPORT.md")
    );
}

#[test]
fn missing_or_case_ambiguous_live_root_document_fails_closed() {
    let missing = SourceRoot::new("live-root-document-missing");
    std::fs::remove_file(missing.path().join("DESIGN.md")).expect("remove required design");
    let failures = crate::audit::source_governance::capture(missing.path()).expect_err("rejected");
    assert!(
        failures
            .iter()
            .any(|failure| failure == "live_root_document_missing:DESIGN.md"),
        "{failures:?}"
    );

    let ambiguous = SourceRoot::new("live-root-document-identity");
    std::fs::remove_file(ambiguous.path().join("DESIGN.md")).expect("remove required design");
    ambiguous.write("design.md", "# Wrong case\n");
    let failures =
        crate::audit::source_governance::capture(ambiguous.path()).expect_err("rejected");
    assert!(
        failures.iter().any(|failure| {
            failure == "governed_source_exact_file_ambiguous:DESIGN.md:design.md"
        }),
        "{failures:?}"
    );
}

#[test]
fn live_root_template_markers_fail_without_scanning_template_assets() {
    let root = SourceRoot::new("live-root-document-template-markers");
    root.write("DESIGN.md", "Describe what this surface should do.\n");
    root.write("FRONTEND.md", "Replace these with real frontend rules.\n");
    root.write("QUALITY_SCORE.md", "Name the release gate here.\n");
    root.write(
        "templates/agent-standards/policy.md",
        "Describe what template consumers should replace.\n",
    );
    let failures = crate::audit::source_governance::capture(root.path()).expect_err("rejected");
    for expected in [
        "live_root_document_unresolved_template:DESIGN.md:describe_what",
        "live_root_document_unresolved_template:FRONTEND.md:replace_these_with",
        "live_root_document_unresolved_template:QUALITY_SCORE.md:name_the_gate",
    ] {
        assert!(
            failures.iter().any(|failure| failure == expected),
            "{expected}: {failures:?}"
        );
    }
    assert!(failures.iter().all(|failure| {
        !failure.starts_with(
            "live_root_document_unresolved_template:templates/agent-standards/policy.md",
        )
    }));
}
