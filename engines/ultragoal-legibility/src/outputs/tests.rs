use super::*;
use crate::{boundaries, syntax};

fn reference(path: &str, symbol: &str) -> SymbolRef {
    SymbolRef {
        path: path.into(),
        symbol: symbol.into(),
    }
}

fn fixture(source: &str) -> (Vec<OutputRow>, BTreeMap<String, Report>, Inventory) {
    let test = "#[test] fn output_preserves_evidence() {}";
    let rows = vec![OutputRow {
        path: "src/output.rs".into(),
        symbol: "Envelope".into(),
        owner: "src/output.rs".into(),
        purpose: "Internal presentation of already measured evidence".into(),
        producer: reference("src/output.rs", "make"),
        validators: vec![],
        tests: vec![reference("tests/output.rs", "output_preserves_evidence")],
    }];
    let reports = BTreeMap::from([(
        "src/output.rs".into(),
        syntax::analyze("src/output.rs", source).unwrap(),
    )]);
    let inventory = Inventory {
        files: BTreeMap::from([
            ("src/output.rs".into(), source.as_bytes().to_vec()),
            ("tests/output.rs".into(), test.as_bytes().to_vec()),
        ]),
        failures: vec![],
        exclusions: vec![],
    };
    (rows, reports, inventory)
}

#[test]
fn exact_output_record_is_permitted_without_claiming_closed_input() {
    let (rows, reports, inventory) = fixture(
        "struct Envelope { data: serde_json::Value } fn make()->Envelope { Envelope { data: serde_json::json!({\"count\":1}) } }",
    );
    let checked = check(&rows, &[], &[], &reports, &inventory);
    assert!(checked.failures.is_empty(), "{:?}", checked.failures);
    assert!(
        boundaries::check(&[], &reports)
            .iter()
            .any(|f| f.contains("raw_authority_outside_boundary"))
    );
    assert!(boundaries::check_with_outputs(&[], &reports, &checked.allowed).is_empty());
    assert!(!permits(
        &checked.allowed,
        "src/output.rs",
        Some("Envelope"),
        "process"
    ));
}

#[test]
fn same_output_registration_cannot_hide_direct_or_transitive_input_effects() {
    for body in [
        "fn make()->Envelope { Envelope { data: serde_json::from_str(\"{}\").unwrap() } }",
        "fn helper() { std::process::Command::new(\"sh\"); } fn make()->Envelope { helper(); Envelope{ data: serde_json::json!({}) } }",
        "fn helper() { std::env::var(\"SECRET\"); } fn make()->Envelope { helper(); Envelope{ data: serde_json::json!({}) } }",
    ] {
        let source = format!("struct Envelope {{ data: serde_json::Value }} {body}");
        let (rows, reports, inventory) = fixture(&source);
        let checked = check(&rows, &[], &[], &reports, &inventory);
        assert!(checked.allowed.is_empty());
        assert!(
            checked
                .failures
                .iter()
                .any(|f| f.starts_with("output_producer_unowned_effect:")),
            "{:?}",
            checked.failures
        );
    }
}

#[test]
fn nonexistent_test_or_producer_cannot_validate_registration() {
    let (mut rows, reports, inventory) =
        fixture("struct Envelope { data: serde_json::Value } fn make()->Envelope { loop {} }");
    rows[0].tests[0].symbol = "invented_test".into();
    let checked = check(&rows, &[], &[], &reports, &inventory);
    assert!(checked.allowed.is_empty());
    assert!(
        checked
            .failures
            .iter()
            .any(|f| f.starts_with("output_behavior_test_unresolved:"))
    );
    rows[0].producer.symbol = "missing_producer".into();
    let checked = check(&rows, &[], &[], &reports, &inventory);
    assert!(
        checked
            .failures
            .iter()
            .any(|f| f.starts_with("output_producer_unresolved:"))
    );
}

#[test]
fn function_cannot_borrow_an_unrelated_pure_producer_to_hide_file_input() {
    let source = "struct Envelope{data:serde_json::Value} fn make()->Envelope{Envelope{data:serde_json::json!({})}} pub fn load(path:&std::path::Path)->String{(std::fs::read_to_string)(path).unwrap()}";
    let (mut rows, reports, inventory) = fixture(source);
    rows[0].symbol = "load".into();
    let checked = check(&rows, &[], &[], &reports, &inventory);
    assert!(checked.allowed.is_empty());
    assert!(
        checked
            .failures
            .iter()
            .any(|f| f.starts_with("output_function_producer_mismatch:"))
    );
    rows[0].producer.symbol = "load".into();
    let checked = check(&rows, &[], &[], &reports, &inventory);
    assert!(checked.allowed.is_empty());
    assert!(
        checked
            .failures
            .iter()
            .any(|f| f.contains("std::fs::read_to_string"))
    );
}

#[test]
fn actual_container_producer_can_own_a_nested_payload_declaration() {
    let source = "struct Envelope{data:serde_json::Value} struct Container{payload:Option<Envelope>} fn make()->Container{Container{payload:None}}";
    let (rows, reports, inventory) = fixture(source);
    let checked = check(&rows, &[], &[], &reports, &inventory);
    assert!(checked.failures.is_empty(), "{:?}", checked.failures);
}

#[test]
fn verified_closed_boundary_is_terminal_but_missing_authority_is_not() {
    let source = "struct Envelope{data:serde_json::Value} struct Error; fn make()->Result<Envelope,Error>{std::fs::read(\"input\"); Ok(Envelope{data:serde_json::json!({})})}";
    let (rows, reports, inventory) = fixture(source);
    let mut boundary = crate::model::BoundaryRow {
        path: "src/output.rs".into(),
        symbol: "make".into(),
        authorities: vec!["filesystem".into()],
        response: "Envelope".into(),
        error: "Error".into(),
        validation: vec![],
        error_field: None,
        error_variant: None,
    };
    let checked = check(&rows, &[boundary], &[], &reports, &inventory);
    assert!(checked.failures.is_empty(), "{:?}", checked.failures);
    boundary = crate::model::BoundaryRow {
        path: "src/output.rs".into(),
        symbol: "make".into(),
        authorities: vec![],
        response: "Envelope".into(),
        error: "Error".into(),
        validation: vec![],
        error_field: None,
        error_variant: None,
    };
    let checked = check(&rows, &[boundary], &[], &reports, &inventory);
    assert!(!checked.failures.is_empty());
}
