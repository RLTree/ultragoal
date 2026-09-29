use crate::{boundaries, syntax};
use std::collections::BTreeMap;

#[test]
fn production_reachable_tests_path_is_not_a_boundary_exemption() {
    let reports = BTreeMap::from([(
        "tests/runtime.rs".into(),
        syntax::analyze(
            "tests/runtime.rs",
            "pub fn execute(){std::process::Command::new(\"sh\").spawn().unwrap();}",
        )
        .unwrap(),
    )]);
    assert!(
        boundaries::check(&[], &reports)
            .iter()
            .any(|f| f.contains("tests/runtime.rs:execute:process"))
    );
}

#[test]
fn includes_composed_macro_paths_and_all_environment_spellings_are_visible() {
    let report = syntax::analyze("src/lib.rs", "include!(\"../examples/runtime.inc\");").unwrap();
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.starts_with("unexpanded_rust_include"))
    );
    let report=syntax::analyze("src/lib.rs","macro_rules! source {($ns:ident,$op:ident)=>{std::$ns::$op(\"HOME\")};} fn helper(){source!(env,var);}").unwrap();
    assert!(
        report
            .failures
            .iter()
            .any(|f| f == "macro_composes_unresolved_authority_path")
    );
    for body in [
        "std::env::vars_os();",
        "std::env::args();",
        "std::env::args_os();",
        "env!(\"HOME\");",
        "option_env!(\"HOME\");",
    ] {
        let report = syntax::analyze("src/lib.rs", &format!("fn helper(){{{body}}}")).unwrap();
        assert!(
            report.authorities.iter().any(|a| a.kind == "environment"),
            "{body}"
        );
    }
}

#[test]
fn parenthesized_and_renamed_authority_paths_cannot_disappear() {
    let report=syntax::analyze("src/lib.rs","extern crate std as operating; fn helper(){operating::env::vars_os(); let _=(std::fs::read)(\"input\");}").unwrap();
    assert!(report.authorities.iter().any(|a| a.kind == "environment"));
    assert!(
        report.functions[0]
            .direct_calls
            .iter()
            .any(|c| c == "std::fs::read")
    );
}
