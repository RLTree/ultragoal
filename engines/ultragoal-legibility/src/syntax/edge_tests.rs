use super::{analyze, analyze_with_object_contract, object_contract_from_source};

#[test]
fn local_declarations_shadow_glob_candidates() {
    let report = analyze(
        "x.rs",
        r#"
        use std::process::*;
        use serde_json::*;
        struct Command;
        impl Command { fn new(_: &str) -> Self { Self } }
        fn from_str(_: &str) {}
        fn local() { Command::new("x"); from_str("{}"); }
    "#,
    )
    .unwrap();
    assert!(report.authorities.is_empty());
}

#[test]
fn mutated_object_definition_cannot_keep_trusted_contract() {
    let source = r#"
        macro_rules! object { ($name:ident { $($(#[$attr:meta])* $field:ident : $ty:ty),* $(,)? }) => {
            #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
            #[serde(deny_unknown_fields)]
            pub struct $name { $($(#[$attr])* pub $field: $ty),* }
            fn hidden() { std::env::var("X"); }
        }; }
        object!(Open { x: String });
    "#;
    assert!(!object_contract_from_source(source));
    let report = analyze_with_object_contract("x.rs", source, true).unwrap();
    assert!(!report.types[0].closed);
    assert!(
        report
            .authorities
            .iter()
            .any(|a| a.kind == "environment" && a.function.is_none())
    );
}

#[test]
fn only_verified_anyhow_alias_gets_its_known_default_error() {
    let report = analyze(
        "x.rs",
        r#"
        use anyhow::Result as Outcome;
        fn imported() -> Outcome<Request> { todo!() }
        fn direct() -> anyhow::Result<Request> { todo!() }
        fn unknown() -> Result<Request> { todo!() }
        fn arbitrary() -> Outcome<serde_json::Value> { todo!() }
        fn sequence() -> anyhow::Result<Vec<Request>> { todo!() }
        mod named { use anyhow::Result; fn imported() -> Result<Request> { todo!() } }
    "#,
    )
    .unwrap();
    assert_eq!(
        report
            .functions
            .iter()
            .map(|f| f.returns_closed_result)
            .collect::<Vec<_>>(),
        [true, true, false, false, true, true]
    );
    assert_eq!(
        report.functions[0].resolved_error.as_deref(),
        Some("anyhow::Error")
    );
    assert_eq!(
        report.functions[1].resolved_error.as_deref(),
        Some("anyhow::Error")
    );
    assert_eq!(report.functions[2].resolved_error, None);
}

#[test]
fn raw_output_type_has_exact_declaration_owner() {
    let report = analyze(
        "x.rs",
        "mod output { struct Telemetry { detail: serde_json::Value } }",
    )
    .unwrap();
    let authority = report
        .authorities
        .iter()
        .find(|a| a.kind == "serde_json_value")
        .unwrap();
    assert_eq!(authority.owner.as_deref(), Some("output::Telemetry"));
    assert_eq!(authority.function, None);
}

#[test]
fn direct_calls_do_not_guess_method_edges() {
    let report = analyze(
        "x.rs",
        "fn x() { validate(); value.validate(); ensure!(true); }",
    )
    .unwrap();
    assert_eq!(report.functions[0].direct_calls, ["validate"]);
    assert!(report.functions[0].calls.contains(&"ensure".into()));
}

#[test]
fn opaque_macro_process_and_parser_inputs_are_visible() {
    let report = analyze(
        "x.rs",
        r#"
        use std::process::Command as Shell;
        fn escaped() {
            opaque!(key => Shell::new("sh"));
            serde_json::json!({"parsed": serde_json::from_str::<serde_json::Value>("{}")});
        }
    "#,
    )
    .unwrap();
    for expected in ["process", "structured_input", "serde_json_value"] {
        assert!(
            report.functions[0]
                .authority_kinds
                .contains(&expected.into()),
            "{expected}"
        );
    }
}

#[test]
fn macro_definition_cannot_hide_module_authority() {
    let report = analyze(
        "x.rs",
        r#"
        macro_rules! hidden { () => { std::process::Command::new("sh") }; }
        fn call() { hidden!(); }
    "#,
    )
    .unwrap();
    assert!(
        report
            .authorities
            .iter()
            .any(|a| a.function.is_none() && a.kind == "process")
    );
}

#[test]
fn literal_macros_are_not_authority() {
    let report = analyze(
        "x.rs",
        r#"
        fn literal() {
            serde_json::json!({"command": "std::process::Command::new", "nested": [1,2]});
            ensure!(true, "std::env::var");
        }
    "#,
    )
    .unwrap();
    assert!(report.authorities.is_empty());
}

#[test]
fn glob_process_and_parser_uses_are_conservative_candidates() {
    let report = analyze(
        "x.rs",
        r#"
        mod process { use std::process::*; fn run() { Command::new("sh"); } }
        mod parser { use serde_json::*; fn read() { from_str::<Value>("{}"); } }
    "#,
    )
    .unwrap();
    assert!(
        report.functions[0]
            .authority_kinds
            .contains(&"process".into())
    );
    assert!(
        report.functions[1]
            .authority_kinds
            .contains(&"structured_input".into())
    );
    assert!(
        report.functions[1]
            .authority_kinds
            .contains(&"serde_json_value".into())
    );
    assert!(
        report
            .dependencies
            .iter()
            .any(|d| d.symbol == "std::process::Command::new")
    );
}

#[test]
fn inherited_object_contract_has_explicit_authority_and_local_shadowing() {
    let inherited =
        analyze_with_object_contract("model/child.rs", "object!(Closed { x: String });", true)
            .unwrap();
    assert!(inherited.types[0].closed);
    assert!(
        inherited
            .dependencies
            .iter()
            .any(|d| d.symbol == "serde::Deserialize" && d.owner.as_deref() == Some("Closed"))
    );
    let shadowed = analyze_with_object_contract(
        "model/child.rs",
        r#"
        macro_rules! object { ($name:ident {$($t:tt)*}) => { struct $name; }; }
        object!(Open { x: String });
    "#,
        true,
    )
    .unwrap();
    assert!(!shadowed.types[0].closed);
    assert!(!object_contract_from_source("fn incomplete("));
    assert!(!object_contract_from_source(
        "object!(Unknown { x: String });"
    ));
}
