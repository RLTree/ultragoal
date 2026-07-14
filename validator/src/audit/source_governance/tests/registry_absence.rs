use super::fixture::SourceRoot;

#[test]
fn missing_dependency_registry_still_reports_every_observed_call_site() {
    let root = SourceRoot::new("dependency-missing-registry");
    root.write(
        "validator/Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2024\"\nlicense = \"MIT\"\n[dependencies]\nsyn = \"2.0.117\"\n",
    );
    root.write(
        "validator/src/audit/source_governance/rust_syntax/adapter.rs",
        "struct RustSyntaxRequest;\nstruct RustSyntaxReport;\nstruct RustSyntaxError;\nfn analyze(input: syn::File) -> Result<RustSyntaxReport, RustSyntaxError> { let _ = input; Err(RustSyntaxError) }\n",
    );
    root.write(
        "validator/src/product.rs",
        "#![allow(dead_code)]\nfn inspect(input: syn::File) { let _ = input; }\n",
    );
    assert!(
        !crate::audit::source_governance::audit(root.path())
            .failures
            .is_empty()
    );
    let failures = crate::audit::plugin::dependency_adapter::failures(root.path());
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("dependency-adapter-registry.json")),
        "{failures:?}"
    );
    for path in [
        "validator/src/audit/source_governance/rust_syntax/adapter.rs",
        "validator/src/product.rs",
    ] {
        assert!(
            failures.iter().any(|failure| {
                failure == &format!("dependency_adapter_unregistered_call_site:syn:{path}")
            }),
            "{path}: {failures:?}"
        );
    }
}
