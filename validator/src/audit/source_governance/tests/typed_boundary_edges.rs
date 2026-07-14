use super::fixture::SourceRoot;
use crate::audit::law::authority_surfaces::{BoundaryRow, failures_for_sources_and_rows};
use crate::audit::source_governance::rust_syntax::AuthorityKind;

const STRING_AUTHORITY: &[AuthorityKind] = &[AuthorityKind::RawString];
const STRING_PATH_AUTHORITIES: &[AuthorityKind] =
    &[AuthorityKind::RawString, AuthorityKind::RawPathBuffer];
const PROCESS_AUTHORITY: &[AuthorityKind] = &[AuthorityKind::Process];

fn production<'a>(
    inventory: &'a crate::audit::source_governance::GovernedInventory,
) -> impl Iterator<Item = &'a crate::audit::source_governance::GovernedSource> {
    inventory
        .sources
        .iter()
        .filter(|source| source.relative.starts_with("validator/src/"))
}

#[test]
fn exact_registered_boundary_accepts_closed_result_and_mutation_removal_rejects() {
    let root = SourceRoot::new("typed-valid");
    root.write(
        "validator/src/input_boundary.rs",
        "use std::string::String;\nstruct Parsed;\nstruct ParseError;\nfn parse(input: String) -> Result<Parsed, ParseError> { let _ = input; Err(ParseError) }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let row = BoundaryRow {
        path: "validator/src/input_boundary.rs",
        symbol: "parse",
        authorities: STRING_AUTHORITY,
    };
    assert!(
        failures_for_sources_and_rows(production(&inventory), std::slice::from_ref(&row))
            .is_empty()
    );
    let removed = failures_for_sources_and_rows(production(&inventory), &[]);
    assert!(
        removed
            .iter()
            .any(|failure| failure.contains("raw_authority_unregistered")),
        "{removed:?}"
    );
}

#[test]
fn exact_process_adapter_accepts_closed_contract_and_mutation_removal_rejects() {
    let root = SourceRoot::new("typed-process-adapter");
    root.write(
        "validator/src/process_adapter.rs",
        "use std::process::Command;\nstruct Request;\nstruct Response;\nstruct AdapterError;\npub(crate) fn run(request: Request) -> Result<Response, AdapterError> { let _ = request; let _ = Command::new(\"true\"); Err(AdapterError) }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let row = BoundaryRow {
        path: "validator/src/process_adapter.rs",
        symbol: "run",
        authorities: PROCESS_AUTHORITY,
    };
    assert!(
        failures_for_sources_and_rows(production(&inventory), std::slice::from_ref(&row))
            .is_empty()
    );
    let removed = failures_for_sources_and_rows(production(&inventory), &[]);
    assert!(
        removed
            .iter()
            .any(|failure| failure.contains("authority=process")),
        "{removed:?}"
    );
}

#[test]
fn parser_decoys_and_open_results_do_not_bless_raw_authority() {
    let root = SourceRoot::new("typed-decoys");
    root.write(
        "validator/src/parser.rs",
        "use serde_json::Value;\nconst PARSER_BOUNDARY: &str = \"marker\";\nfn parse(value: Value) -> Result<Value, String> { serde_json::from_value::<Value>(value).map_err(|error| error.to_string()) }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let failures = failures_for_sources_and_rows(production(&inventory), &[]);
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("serde_json_value")),
        "{failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|failure| failure.contains("authority=string")),
        "{failures:?}"
    );
}

#[test]
fn harmless_local_and_output_strings_are_not_semantic_authority() {
    let root = SourceRoot::new("typed-harmless-text");
    root.write(
        "validator/src/rendering.rs",
        "use std::string::String;\nfn render() -> String { let text: String = format!(\"{}\", 1); text }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let failures = failures_for_sources_and_rows(production(&inventory), &[]);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn closed_record_string_fields_are_typed_but_downstream_reparse_is_rejected() {
    let root = SourceRoot::new("typed-semantic-text");
    root.write(
        "validator/src/external_record.rs",
        "use serde_json::Value;\npub(crate) struct ExternalRecord { pub(crate) candidate_identity: String }\nfn consume(value: Value) { let _: Value = serde_json::from_value::<Value>(value).expect(\"value\"); }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let failures = failures_for_sources_and_rows(production(&inventory), &[]).join("\n");
    assert!(!failures.contains("authority=string"), "{failures}");
    assert!(failures.contains("structured_input"), "{failures}");
    assert!(failures.contains("serde_json_value"), "{failures}");
}

#[test]
fn imported_maps_values_paths_process_and_environment_are_ast_authority() {
    let root = SourceRoot::new("typed-kinds");
    root.write(
        "validator/src/authority_boundary.rs",
        "use serde_json::{Map, Value};\nuse std::collections::HashMap;\nuse std::path::PathBuf;\nuse toml::Value as TomlValue;\npub(crate) fn consume(one: Map<String, Value>, two: HashMap<String, Value>, three: TomlValue, path: PathBuf) { let _ = (one, two, three, path); let _ = std::process::Command::new(\"true\"); let _ = std::env::var(\"HOME\"); }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let failures = failures_for_sources_and_rows(production(&inventory), &[]).join("\n");
    for kind in [
        "serde_json_map",
        "serde_json_value",
        "string_value_map",
        "toml_value",
        "path_buf",
        "process",
        "environment",
    ] {
        assert!(failures.contains(kind), "missing {kind}: {failures}");
    }
}

#[test]
fn registry_rejects_unknown_duplicate_conflicting_and_open_rows() {
    let root = SourceRoot::new("typed-registry");
    root.write(
        "validator/src/input_boundary.rs",
        "use std::{path::PathBuf, string::String};\nfn parse(input: String, path: PathBuf) -> Result<String, String> { let _ = path; Ok(input) }\n",
    );
    let inventory = crate::audit::source_governance::capture(root.path()).expect("inventory");
    let string = BoundaryRow {
        path: "validator/src/input_boundary.rs",
        symbol: "parse",
        authorities: STRING_AUTHORITY,
    };
    let conflict = BoundaryRow {
        path: "validator/src/input_boundary.rs",
        symbol: "parse",
        authorities: STRING_PATH_AUTHORITIES,
    };
    let unknown = BoundaryRow {
        path: "validator/src/missing.rs",
        symbol: "parse",
        authorities: STRING_AUTHORITY,
    };
    let failures = failures_for_sources_and_rows(
        production(&inventory),
        &[string.clone(), string, conflict, unknown],
    );
    for expected in [
        "registry_duplicate",
        "registry_conflict",
        "unknown_path",
        "open_result",
    ] {
        assert!(
            failures.iter().any(|failure| failure.contains(expected)),
            "{expected}: {failures:?}"
        );
    }
}
