use super::fixture::SourceRoot;
use crate::audit::plugin::dependency_adapter::{
    BoundaryKind, DeclarationSite, DependencyProfile, DependencyRegistry, DependencyRow,
    DirectDependency, OperationalPolicy, ProfileContract, failures_for_test,
};

fn dependency(crate_name: &str) -> DirectDependency {
    DirectDependency {
        crate_name: crate_name.to_string(),
        version_requirement: "2.0.117".to_string(),
    }
}

fn runtime_profile(module: &str) -> DependencyProfile {
    DependencyProfile {
        adapter_id: "rust_source_syntax".to_string(),
        boundary_kind: BoundaryKind::TypedParser,
        contract: ProfileContract::OwnedAdapter {
            module: module.to_string(),
            request: "RustSyntaxRequest".to_string(),
            response: "RustSyntaxReport".to_string(),
            error: "RustSyntaxError".to_string(),
        },
        timeout: OperationalPolicy::NotApplicable {
            rationale: "pure parsing is bounded by governed source bytes".to_string(),
        },
        retry: OperationalPolicy::NotApplicable {
            rationale: "deterministic parse failures cannot benefit from retry".to_string(),
        },
        cache_invalidation_inputs: vec![
            "rust_syntax_policy_version".to_string(),
            "source_bytes_sha256".to_string(),
            "source_path".to_string(),
        ],
    }
}

fn row(crate_name: &str) -> DependencyRow {
    DependencyRow {
        crate_name: crate_name.to_string(),
        version_requirement: "2.0.117".to_string(),
        owner: "source_governance_rust_syntax".to_string(),
        purpose: "parse Rust into closed syntax authority records".to_string(),
        upstream_docs: format!("https://docs.rs/{crate_name}/2.0.117/{crate_name}/"),
        profiles: vec![runtime_profile(
            "validator/src/audit/source_governance/rust_syntax/adapter.rs",
        )],
    }
}

fn registry(rows: Vec<DependencyRow>) -> DependencyRegistry {
    DependencyRegistry {
        schema_version: "dependency-adapter-registry-v2".to_string(),
        rows,
    }
}

fn adapter_root(label: &str) -> SourceRoot {
    let root = SourceRoot::new(label);
    root.write(
        "validator/src/audit/source_governance/rust_syntax/adapter.rs",
        "struct RustSyntaxRequest;\nstruct RustSyntaxReport;\nstruct RustSyntaxError;\nfn analyze(input: syn::File) -> Result<RustSyntaxReport, RustSyntaxError> { let _ = input; Err(RustSyntaxError) }\n",
    );
    root
}

#[test]
fn exact_adapter_registry_accepts_closed_call_site() {
    let root = adapter_root("dependency-valid");
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    assert!(
        failures_for_test(
            &[dependency("syn")],
            &registry(vec![row("syn")]),
            &inventory
        )
        .is_empty()
    );
}

#[test]
fn registry_rejects_missing_duplicate_stale_and_version_drift() {
    let root = adapter_root("dependency-rows");
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let mut drift = row("syn");
    drift.version_requirement = "2.0.116".to_string();
    let stale = row("unknown");
    let failures = failures_for_test(
        &[dependency("syn")],
        &registry(vec![drift.clone(), drift, stale]),
        &inventory,
    );
    for expected in ["registry_duplicate", "registry_stale", "version_mismatch"] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
    let missing = failures_for_test(&[dependency("syn")], &registry(vec![]), &inventory);
    assert!(
        missing
            .iter()
            .any(|failure| failure.contains("registry_missing")),
        "{missing:?}"
    );
}

#[test]
fn bypass_and_incomplete_operational_contract_fail() {
    let root = adapter_root("dependency-bypass");
    root.write(
        "validator/src/product.rs",
        "fn bypass(input: syn::File) { let _ = input; }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let mut incomplete = row("syn");
    incomplete.profiles[0].timeout = OperationalPolicy::NotApplicable {
        rationale: String::new(),
    };
    incomplete.profiles[0].cache_invalidation_inputs = vec!["source_path".to_string(); 2];
    let failures = failures_for_test(
        &[dependency("syn")],
        &registry(vec![incomplete]),
        &inventory,
    );
    for expected in [
        "direct_bypass",
        "operational_policy_missing",
        "cache_inputs_invalid",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
}

#[test]
fn broad_module_and_decoy_type_ownership_do_not_bless_use() {
    let root = adapter_root("dependency-decoy");
    root.write(
        "validator/src/audit/source_governance/rust_syntax/decoy.rs",
        "struct RustSyntaxRequest;\nstruct RustSyntaxReport;\nstruct RustSyntaxError;\n",
    );
    root.write(
        "validator/src/audit/source_governance/rust_syntax/adapter.rs",
        "fn analyze(input: syn::File) { let _ = input; }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let mut broad = row("syn");
    broad.profiles[0].contract = ProfileContract::OwnedAdapter {
        module: "validator/src/".to_string(),
        request: "RustSyntaxRequest".to_string(),
        response: "RustSyntaxReport".to_string(),
        error: "RustSyntaxError".to_string(),
    };
    let failures = failures_for_test(&[dependency("syn")], &registry(vec![broad]), &inventory);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("owned_contract_invalid")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("direct_bypass")),
        "{failures:?}"
    );
}

#[test]
fn declarative_contract_profiles_require_exact_domain_symbols_without_wrappers() {
    let root = SourceRoot::new("dependency-declaration");
    root.write(
        "validator/src/product_contract.rs",
        "use serde::Deserialize;\n#[derive(Deserialize)] struct ProductContract { name: String }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let mut serde_row = row("serde");
    serde_row.profiles = vec![DependencyProfile {
        adapter_id: "product_contract_derive".to_string(),
        boundary_kind: BoundaryKind::TypedContractDerive,
        contract: ProfileContract::TypedDeclarations {
            declarations: vec![DeclarationSite {
                module: "validator/src/product_contract.rs".to_string(),
                symbols: vec!["ProductContract".to_string()],
            }],
        },
        timeout: OperationalPolicy::NotApplicable {
            rationale: "derive expansion performs no runtime external operation".to_string(),
        },
        retry: OperationalPolicy::NotApplicable {
            rationale: "declarative compilation cannot benefit from runtime retry".to_string(),
        },
        cache_invalidation_inputs: vec![
            "compiler_version".to_string(),
            "source_bytes_sha256".to_string(),
        ],
    }];
    assert!(
        failures_for_test(
            &[dependency("serde")],
            &registry(vec![serde_row]),
            &inventory
        )
        .is_empty()
    );
}

#[test]
fn registry_deserialization_rejects_unknown_rows() {
    let error = serde_json::from_str::<DependencyRegistry>(
        r#"{"schema_version":"dependency-adapter-registry-v2","rows":[],"unknown":true}"#,
    )
    .expect_err("unknown field rejected");
    assert!(error.to_string().contains("unknown field"));
}
