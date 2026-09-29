use super::analyze;

#[test]
fn module_authority_has_no_boundary_owner() {
    let report = analyze("input.rs", "static RAW: () = { std::process::Command::new(\"sh\"); }; struct Open { raw: serde_json::Value }").unwrap();
    assert!(
        report
            .authorities
            .iter()
            .any(|a| a.function.is_none() && a.kind == "process")
    );
    assert!(
        report
            .authorities
            .iter()
            .any(|a| a.function.is_none() && a.kind == "serde_json_value")
    );
    assert!(!report.types[0].closed);
}

#[test]
fn imported_process_cannot_bypass_qualified_owner() {
    let report = analyze("input.rs", r#"
        mod allowed { pub fn run() {} }
        mod escaped { use std::process::Command as Shell; pub fn run() { Shell::new("sh").spawn(); } }
    "#).unwrap();
    let run = report
        .functions
        .iter()
        .find(|f| f.name == "escaped::run")
        .unwrap();
    assert!(run.authority_kinds.contains(&"process".into()));
    assert!(
        report
            .dependencies
            .iter()
            .any(|d| d.symbol == "std::process::Command::new"
                && d.owner.as_deref() == Some("escaped::run"))
    );
    assert!(
        !report
            .functions
            .iter()
            .find(|f| f.name == "allowed::run")
            .unwrap()
            .authority_kinds
            .contains(&"process".into())
    );
}

#[test]
fn imported_parser_and_local_alias_are_resolved() {
    let report = analyze(
        "input.rs",
        r#"
        use serde_json as json;
        use json::from_str as decode;
        fn ingest(raw: &str) -> Result<Request, Error> {
            let result = decode(raw)?; validate_request(&result)?; Ok(result)
        }
    "#,
    )
    .unwrap();
    let function = &report.functions[0];
    assert!(
        function
            .authority_kinds
            .contains(&"structured_input".into())
    );
    assert!(
        function
            .validation_calls
            .contains(&"validate_request".into())
    );
    assert!(function.calls.contains(&"serde_json::from_str".into()));
    assert!(function.returns_closed_result);
}

#[test]
fn serde_closed_control_and_open_payload() {
    let report = analyze("input.rs", r#"
        use serde::Deserialize;
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Closed { text: String }
        #[derive(Deserialize)] struct UnknownFields { text: String }
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Arbitrary { payload: serde_json::Value }
        #[derive(Deserialize)] #[serde(deny_unknown_fields)] struct Flattened { #[serde(flatten)] payload: Closed }
        struct Wrapped { inner: Arbitrary }
    "#).unwrap();
    for ty in &report.types {
        assert_eq!(ty.closed, ty.name == "Closed", "{}", ty.name);
    }
    assert!(
        report
            .dependencies
            .iter()
            .any(|d| d.symbol == "serde::Deserialize" && d.owner.as_deref() == Some("Closed"))
    );
}

#[test]
fn preserves_named_error_rule_with_approved_sequence_adaptation() {
    let report = analyze(
        "input.rs",
        r#"
        fn valid() -> Result<Request, anyhow::Error> { todo!() }
        fn alias() -> Result<Request> { todo!() }
        fn raw() -> Result<serde_json::Value, Error> { todo!() }
        fn vector() -> Result<Vec<Request>, Error> { todo!() }
        fn reference() -> Result<&Request, Error> { todo!() }
    "#,
    )
    .unwrap();
    assert_eq!(
        report
            .functions
            .iter()
            .map(|f| f.returns_closed_result)
            .collect::<Vec<_>>(),
        [true, false, false, true, false]
    );
    assert!(
        report.functions[2]
            .output_identifiers
            .contains(&"serde_json::Value".into())
    );
}

#[test]
fn imports_do_not_leak_across_sibling_modules_or_blocks() {
    let report = analyze("input.rs", r#"
        mod left { use serde_json::from_str as decode; fn f() { decode(""); } }
        mod right { fn f() { decode(""); { use toml::from_str as decode; decode(""); } decode(""); } }
    "#).unwrap();
    let right = report
        .functions
        .iter()
        .find(|f| f.name == "right::f")
        .unwrap();
    assert!(!right.calls.contains(&"serde_json::from_str".into()));
    assert!(right.calls.contains(&"toml::from_str".into()));
    assert!(right.calls.contains(&"decode".into()));
}

#[test]
fn non_test_cfg_branches_are_not_erased() {
    let report = analyze(
        "input.rs",
        r#"
        #[cfg(test)] mod tests { fn f() { std::process::Command::new("test"); } }
        #[cfg(not(test))] fn production() { std::process::Command::new("prod"); }
        #[cfg(any(test, feature="x"))] fn conditional() { std::env::var("X"); }
    "#,
    )
    .unwrap();
    assert_eq!(report.functions.len(), 2);
    assert!(report.functions.iter().any(|f| f.name == "conditional"));
}

#[test]
fn signatures_and_parser_generic_types_match_original_contexts() {
    let report = analyze(
        "input.rs",
        r#"
        pub fn entry(value: String) { let x: serde_json::Value = todo!(); }
        fn helper(value: String) { let x: serde_json::Value = todo!(); }
        fn parse(raw: String) { serde_json::from_str::<serde_json::Value>(&raw); }
    "#,
    )
    .unwrap();
    assert_eq!(report.functions[0].authority_kinds, ["string"]);
    assert!(report.functions[1].authority_kinds.is_empty());
    assert!(
        report.functions[2]
            .authority_kinds
            .contains(&"serde_json_value".into())
    );
}

#[test]
fn same_method_names_are_qualified_by_type() {
    let report = analyze(
        "input.rs",
        "struct A; struct B; impl A { fn run() {} } impl B { fn run() { std::env::var(\"X\"); } }",
    )
    .unwrap();
    assert_eq!(report.functions[0].name, "A::run");
    assert_eq!(report.functions[1].name, "B::run");
}

#[test]
fn object_requires_recovered_contract_and_refuses_value() {
    let source = r#"
        use serde::{Serialize, Deserialize};
        macro_rules! object { ($name:ident { $($(#[$attr:meta])* $field:ident : $ty:ty),* $(,)? }) => {
            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
            #[serde(deny_unknown_fields)]
            pub struct $name { $($(#[$attr])* pub $field: $ty),* }
        }; }
        object!(Closed { text: String });
        object!(Open { value: serde_json::Value });
    "#;
    let report = analyze("model.rs", source).unwrap();
    assert!(report.types[0].closed);
    assert!(!report.types[1].closed);
    let untrusted = analyze("model.rs", "object!(Unknown { text: String });").unwrap();
    assert!(!untrusted.types[0].closed);
}

#[test]
fn macros_visit_parseable_inputs_and_report_opaque_inputs() {
    let report = analyze(
        "input.rs",
        r#"fn f() { ensure!(std::env::var("X").is_ok(), "bad"); weird!(x => y); }"#,
    )
    .unwrap();
    assert!(
        report.functions[0]
            .authority_kinds
            .contains(&"environment".into())
    );
    assert!(
        report.functions[0]
            .validation_calls
            .contains(&"ensure".into())
    );
    assert!(
        report
            .limitations
            .iter()
            .any(|l| l.contains("Unexpanded macro: weird"))
    );
}

#[test]
fn source_errors_include_path() {
    assert!(
        analyze("broken.rs", "fn broken(")
            .unwrap_err()
            .starts_with("broken.rs:")
    );
}
