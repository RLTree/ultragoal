use super::{analyze, analyze_with_object_contract};

#[test]
fn unit_and_recursive_named_response_wrappers_are_closed() {
    let report = analyze(
        "x.rs",
        r#"
        fn unit() -> Result<(), Error> { todo!() }
        fn optional() -> Result<Option<Reply>, Error> { todo!() }
        fn sequence() -> Result<Vec<Reply>, Error> { todo!() }
        fn nested() -> anyhow::Result<Option<Vec<Reply>>> { todo!() }
        fn raw_sequence() -> Result<Vec<serde_json::Value>, Error> { todo!() }
        fn raw_option() -> Result<Option<String>, Error> { todo!() }
        fn raw_path() -> Result<Vec<Option<std::path::PathBuf>>, Error> { todo!() }
        fn named_map() -> Result<Option<std::collections::BTreeMap<String, Reply>>, Error> { todo!() }
        fn raw_error() -> Result<(), String> { todo!() }
        fn unnamed_error() -> Result<(), Option<Error>> { todo!() }
        fn plain_domain() -> Grade { todo!() }
    "#,
    )
    .unwrap();
    assert_eq!(
        report
            .functions
            .iter()
            .map(|f| f.returns_closed_result)
            .collect::<Vec<_>>(),
        [
            true, true, true, true, false, false, false, true, false, false, false
        ]
    );
}

#[test]
fn declaration_fields_and_enum_variants_are_ast_evidence() {
    let report = analyze(
        "x.rs",
        r#"
        enum Verdict { Pass, Fail, Ungradable }
        struct Grade { evaluator_error: Option<String>, verdict: Verdict }
        fn grade() -> Grade { todo!() }
    "#,
    )
    .unwrap();
    assert_eq!(report.types[0].variants, ["Pass", "Fail", "Ungradable"]);
    assert_eq!(
        report.types[1].fields,
        [
            ("evaluator_error".into(), "Option < String >".into()),
            ("verdict".into(), "Verdict".into())
        ]
    );
    assert!(!report.functions[0].returns_closed_result);
}

#[test]
fn verified_object_fields_remain_declared_error_field_evidence() {
    let report = analyze_with_object_contract(
        "model/grade.rs",
        "object!(Grade { evaluator_error: Option<String>, verdict: Verdict });",
        true,
    )
    .unwrap();
    assert!(report.types[0].closed);
    assert_eq!(
        report.types[0].fields[0],
        ("evaluator_error".into(), "Option < String >".into())
    );
}
