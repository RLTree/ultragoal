use super::{
    analyze, analyze_with_context, analyze_with_full_context, module_anyhow_aliases,
    module_type_aliases,
};

#[test]
fn source_verified_parent_aliases_are_inherited_and_shadowable() {
    let aliases = module_type_aliases(
        "use anyhow::Result; type CommandResult = Result<Response>;",
        &[],
        &[],
    )
    .unwrap();
    assert_eq!(aliases[0].0, "CommandResult");
    let good = analyze_with_full_context(
        "child.rs",
        "use super::*; fn read() -> CommandResult { todo!() }",
        false,
        &[],
        &aliases,
    )
    .unwrap();
    assert!(good.functions[0].returns_closed_result);
    assert_eq!(
        good.functions[0].resolved_error.as_deref(),
        Some("anyhow::Error")
    );
    let shadow = analyze_with_full_context(
        "child.rs",
        "type CommandResult = serde_json::Value; fn read() -> CommandResult { todo!() }",
        false,
        &[],
        &aliases,
    )
    .unwrap();
    assert!(!shadow.functions[0].returns_closed_result);
    let exact = vec![("super::CommandResult".into(), aliases[0].1.clone())];
    let imported = analyze_with_full_context(
        "child.rs",
        "use super::CommandResult as Outcome; fn read() -> Outcome { todo!() }",
        false,
        &[],
        &exact,
    )
    .unwrap();
    assert!(imported.functions[0].returns_closed_result);
}

#[test]
fn inherited_alias_payload_shape_is_not_assumed_closed() {
    for source in [
        "anyhow::Result<serde_json::Value>",
        "anyhow::Result<(String, String)>",
        "serde_json::Value",
    ] {
        let aliases = vec![("CommandResult".into(), source.into())];
        let report = analyze_with_full_context(
            "x.rs",
            "fn raw() -> CommandResult { todo!() }",
            false,
            &[],
            &aliases,
        )
        .unwrap();
        assert!(!report.functions[0].returns_closed_result, "{source}");
    }
}

#[test]
fn inherited_alias_is_verified_and_local_alias_shadow_refused() {
    let names = vec!["Result".into()];
    let good = analyze_with_context(
        "child.rs",
        "use super::*; fn read() -> Result<Response> { todo!() }",
        false,
        &names,
    )
    .unwrap();
    assert!(good.functions[0].returns_closed_result);
    assert_eq!(
        good.functions[0].resolved_error.as_deref(),
        Some("anyhow::Error")
    );
    let shadowed = analyze_with_context(
        "child.rs",
        "type Result = serde_json::Value; fn read() -> Result { todo!() }",
        false,
        &names,
    )
    .unwrap();
    assert!(!shadowed.functions[0].returns_closed_result);
    let unknown = analyze_with_context(
        "child.rs",
        "use other::Result; fn read() -> Result<Response> { todo!() }",
        false,
        &names,
    )
    .unwrap();
    assert!(!unknown.functions[0].returns_closed_result);
}

#[test]
fn exact_cross_file_import_path_can_be_verified() {
    let report = analyze_with_context(
        "child.rs",
        "use crate::parent::Result as Outcome; fn read() -> Outcome<Response> { todo!() }",
        false,
        &["crate::parent::Result".into()],
    )
    .unwrap();
    assert!(report.functions[0].returns_closed_result);
    let names = module_anyhow_aliases("pub use anyhow::Result as Outcome;", &[]).unwrap();
    assert_eq!(names, ["Outcome"]);
    assert!(
        module_anyhow_aliases("type Result = serde_json::Value;", &["Result".into()])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn local_result_aliases_expand_actual_rhs_recursively() {
    let report = analyze(
        "x.rs",
        r#"
        use anyhow::Result;
        type CommandResult = Result<NamedResponse>;
        type Recursive = CommandResult;
        type Payload = serde_json::Value;
        type RawResult = Result<Payload>;
        type TupleResult = Result<(String, String)>;
        fn good() -> Recursive { todo!() }
        fn raw() -> RawResult { todo!() }
        fn tuple() -> TupleResult { todo!() }
    "#,
    )
    .unwrap();
    assert_eq!(
        report
            .functions
            .iter()
            .map(|f| f.returns_closed_result)
            .collect::<Vec<_>>(),
        [true, false, false]
    );
    assert!(
        report.functions[0]
            .output_identifiers
            .contains(&"NamedResponse".into())
    );
    assert_eq!(
        report.functions[0].resolved_error.as_deref(),
        Some("anyhow::Error")
    );
}

#[test]
fn cyclic_aliases_fail_and_exact_generic_aliases_substitute() {
    let report = analyze("x.rs", "type A = B; type B = A; type Generic<T> = anyhow::Result<T>; fn cycle() -> A { todo!() } fn generic() -> Generic<Named> { todo!() }").unwrap();
    assert!(!report.functions[0].returns_closed_result);
    assert!(report.functions[1].returns_closed_result);
    assert!(
        report
            .limitations
            .iter()
            .any(|l| l.contains("Cyclic type alias"))
    );
}
