use crate::{boundaries, model::BoundaryRow, syntax};
use std::collections::BTreeMap;

#[test]
fn direct_domain_outcome_needs_real_registered_error_field() {
    let source = "struct Grade { evaluator_error: Option<String> } fn grade()->Grade { serde_json::from_str::<Grade>(\"{}\").unwrap() }";
    let reports = BTreeMap::from([(
        "src/grades.rs".into(),
        syntax::analyze("src/grades.rs", source).unwrap(),
    )]);
    let mut row = BoundaryRow {
        path: "src/grades.rs".into(),
        symbol: "grade".into(),
        authorities: vec!["structured_input".into()],
        response: "Grade".into(),
        error: String::new(),
        validation: vec!["serde_json::from_str".into()],
        error_field: None,
        error_variant: None,
    };
    assert!(
        boundaries::check(&[row], &reports)
            .iter()
            .any(|f| f.starts_with("boundary_open_result:"))
    );
    row = BoundaryRow {
        path: "src/grades.rs".into(),
        symbol: "grade".into(),
        authorities: vec!["structured_input".into()],
        response: "Grade".into(),
        error: String::new(),
        validation: vec!["serde_json::from_str".into()],
        error_field: Some("evaluator_error".into()),
        error_variant: None,
    };
    assert!(boundaries::check(&[row], &reports).is_empty());
    let missing = source.replace("evaluator_error", "unrelated_message");
    let reports = BTreeMap::from([(
        "src/grades.rs".into(),
        syntax::analyze("src/grades.rs", &missing).unwrap(),
    )]);
    let row = BoundaryRow {
        path: "src/grades.rs".into(),
        symbol: "grade".into(),
        authorities: vec!["structured_input".into()],
        response: "Grade".into(),
        error: String::new(),
        validation: vec![],
        error_field: Some("evaluator_error".into()),
        error_variant: None,
    };
    assert!(
        boundaries::check(&[row], &reports)
            .iter()
            .any(|f| f.starts_with("boundary_open_result:"))
    );
}

#[test]
fn direct_domain_enum_requires_the_actual_error_variant() {
    let reports = BTreeMap::from([("src/parser.rs".into(),syntax::analyze("src/parser.rs", "enum Parsed { Valid, Invalid } fn parse()->Parsed { serde_json::from_str(\"{}\").unwrap() }").unwrap())]);
    let row = BoundaryRow {
        path: "src/parser.rs".into(),
        symbol: "parse".into(),
        authorities: vec!["structured_input".into()],
        response: "Parsed".into(),
        error: String::new(),
        validation: vec![],
        error_field: None,
        error_variant: Some("Invalid".into()),
    };
    assert!(boundaries::check(&[row], &reports).is_empty());
}
