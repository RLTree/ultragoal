use crate::{boundaries, dependencies, inventory, metadata, model::*, syntax};
use std::{
    collections::BTreeMap,
    fs,
    sync::atomic::{AtomicUsize, Ordering},
};

const VALID: &str = "struct Request; struct Response; struct Error; fn parse(raw: &str) -> Result<Response, Error> { serde_json::from_str(raw) }";

fn boundary() -> BoundaryRow {
    BoundaryRow {
        error_field: None,
        error_variant: None,
        path: "src/parser.rs".into(),
        symbol: "parse".into(),
        authorities: vec!["structured_input".into()],
        response: "Response".into(),
        error: "Error".into(),
        validation: vec!["serde_json::from_str".into()],
    }
}

fn dependency() -> DependencyRow {
    DependencyRow {
        profile_registries: vec![],
        manifest: "Cargo.toml".into(),
        crate_name: "serde_json".into(),
        version_requirement: "=1.0.151".into(),
        owner: "src/parser.rs".into(),
        purpose: "Decode external requests into the owned schema".into(),
        upstream_docs: "https://docs.rs/serde_json".into(),
        profiles: vec![DependencyProfile {
            adapter_id: "request-parser".into(),
            boundary_kind: "typed_parser".into(),
            contract: ProfileContract::OwnedAdapter {
                module: "src/parser.rs".into(),
                request: "Request".into(),
                response: "Response".into(),
                error: "Error".into(),
            },
            timeout: OperationalPolicy::NotApplicable {
                rationale: "In-memory bounded parse".into(),
            },
            retry: OperationalPolicy::NotApplicable {
                rationale: "Malformed requests are refused".into(),
            },
            cache_invalidation_inputs: vec!["No cache; request bytes and schema version".into()],
        }],
    }
}

fn reports() -> BTreeMap<String, syntax::Report> {
    BTreeMap::from([(
        "src/parser.rs".into(),
        syntax::analyze("src/parser.rs", VALID).unwrap(),
    )])
}

#[test]
fn reviewed_closed_boundary_passes_and_direct_unregistered_bypass_fails() {
    let mut reports = reports();
    let metadata = metadata::Metadata {
        dependencies: vec![metadata::DirectDependency {
            manifest: "Cargo.toml".into(),
            crate_name: "serde_json".into(),
            requirement: "=1.0.151".into(),
        }],
        entrypoints: vec![],
        verified_locks: vec![],
        failures: vec![],
    };
    let inventory = inventory::Inventory {
        files: BTreeMap::from([("src/parser.rs".into(), VALID.as_bytes().to_vec())]),
        exclusions: vec![],
        failures: vec![],
    };
    assert!(boundaries::check(&[boundary()], &reports).is_empty());
    assert!(
        dependencies::check(
            &[dependency()],
            &metadata,
            &reports,
            &inventory,
            &[boundary()]
        )
        .is_empty()
    );
    reports.insert(
        "src/bypass.rs".into(),
        syntax::analyze(
            "src/bypass.rs",
            "fn bypass(s:&str)->Result<Response,Error>{serde_json::from_str(s)}",
        )
        .unwrap(),
    );
    assert!(
        boundaries::check(&[boundary()], &reports)
            .iter()
            .any(|f| f.starts_with("raw_authority_unregistered:src/bypass.rs"))
    );
    assert!(
        dependencies::check(
            &[dependency()],
            &metadata,
            &reports,
            &inventory,
            &[boundary()]
        )
        .iter()
        .any(|f| f.starts_with("dependency_direct_bypass:src/bypass.rs"))
    );
}

#[test]
fn nominal_registry_cannot_make_open_json_result_closed() {
    let mut reports = reports();
    reports.insert(
        "src/parser.rs".into(),
        syntax::analyze(
            "src/parser.rs",
            "fn parse(raw:&str)->Result<serde_json::Value,Error>{serde_json::from_str(raw)}",
        )
        .unwrap(),
    );
    let failures = boundaries::check(&[boundary()], &reports);
    assert!(
        failures
            .iter()
            .any(|f| f.starts_with("boundary_open_result:"))
    );
    assert!(
        failures
            .iter()
            .any(|f| f.starts_with("boundary_declared_result_not_returned:"))
    );
}

#[test]
fn changed_dependency_version_and_invented_validation_fail() {
    let mut row = boundary();
    row.validation = vec!["validate_request".into()];
    assert!(
        boundaries::check(&[row], &reports())
            .iter()
            .any(|f| f.starts_with("boundary_validation_call_missing:"))
    );
    let metadata = metadata::Metadata {
        dependencies: vec![],
        entrypoints: vec![],
        verified_locks: vec![],
        failures: vec![],
    };
    let inventory = inventory::Inventory {
        files: BTreeMap::new(),
        exclusions: vec![],
        failures: vec![],
    };
    assert!(
        dependencies::check(
            &[dependency()],
            &metadata,
            &reports(),
            &inventory,
            &[boundary()]
        )
        .iter()
        .any(|f| f.starts_with("dependency_version_or_manifest_mismatch:"))
    );
}

#[test]
fn function_adapter_does_not_authorize_another_function_in_same_file() {
    let source =
        format!("{VALID} fn bypass(raw:&str)->Result<Response,Error>{{serde_json::from_str(raw)}}");
    let reports = BTreeMap::from([(
        "src/parser.rs".into(),
        syntax::analyze("src/parser.rs", &source).unwrap(),
    )]);
    let metadata = metadata::Metadata {
        dependencies: vec![metadata::DirectDependency {
            manifest: "Cargo.toml".into(),
            crate_name: "serde_json".into(),
            requirement: "=1.0.151".into(),
        }],
        entrypoints: vec![],
        verified_locks: vec![],
        failures: vec![],
    };
    let inventory = inventory::Inventory {
        files: BTreeMap::from([("src/parser.rs".into(), source.into_bytes())]),
        exclusions: vec![],
        failures: vec![],
    };
    let mut dependency = dependency();
    dependency.profiles[0].contract = ProfileContract::FunctionAdapters {
        module: "src/parser.rs".into(),
        functions: vec!["parse".into()],
    };
    let failures = dependencies::check(
        &[dependency],
        &metadata,
        &reports,
        &inventory,
        &[boundary()],
    );
    assert!(
        failures
            .iter()
            .any(|f| f.starts_with("dependency_direct_bypass:") && f.ends_with(":bypass"))
    );
}

#[test]
fn physical_cap_has_real_250_251_boundary_and_no_test_exemption() {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "legibility-cap-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    for folder in [
        "src",
        "tests",
        "scripts",
        "ui",
        "skills",
        "docs",
        "tools/legibility",
    ] {
        fs::create_dir_all(root.join(folder)).unwrap();
    }
    fs::write(root.join("tests/exact.rs"), "// authored\n".repeat(250)).unwrap();
    fs::write(root.join("tests/over.rs"), "// authored\n".repeat(251)).unwrap();
    let inventory = inventory::collect(&root);
    assert!(
        !inventory
            .failures
            .iter()
            .any(|f| f.starts_with("authored_line_cap:tests/exact.rs"))
    );
    assert!(
        inventory
            .failures
            .iter()
            .any(|f| f.starts_with("authored_line_cap:tests/over.rs:251"))
    );
    fs::remove_dir_all(root).unwrap();
}
