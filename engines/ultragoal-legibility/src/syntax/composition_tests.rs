use super::*;
#[test]
fn precise_named_map_and_tuple_compositions_preserve_raw_refusal() {
    for (shape, expected) in [
        ("BTreeMap<String,CommandResult>", true),
        ("HashMap<String,Value>", false),
        ("(First,Second)", true),
        ("(First,Value)", false),
    ] {
        let source = format!("fn read()->Result<{shape},Error>{{loop{{}}}}");
        assert_eq!(
            analyze("src/lib.rs", &source).unwrap().functions[0].returns_closed_result,
            expected,
            "{shape}"
        );
    }
}
#[test]
fn immutable_local_closure_calls_do_not_become_glob_calls_and_effects_remain_visible() {
    let source = "use crate::model::*; fn run(){let local=||{std::env::var(\"HOME\")};local();}";
    let report = analyze("src/lib.rs", source).unwrap();
    assert!(
        !report.functions[0]
            .direct_calls
            .iter()
            .any(|c| c == "local" || c == "crate::model::local")
    );
    assert!(report.authorities.iter().any(|a| a.kind == "environment"));
}
