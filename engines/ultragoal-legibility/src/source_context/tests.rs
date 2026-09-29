use super::*;

fn analyze(parent: &str, child: &str) -> crate::syntax::Report {
    let files = BTreeMap::from([
        ("src/commands/mod.rs".into(), parent.as_bytes().to_vec()),
        ("src/commands/run.rs".into(), child.as_bytes().to_vec()),
    ]);
    let contexts = contexts(&files);
    let context = &contexts["src/commands/run.rs"];
    syntax::analyze_with_full_context(
        "src/commands/run.rs",
        child,
        false,
        &context.anyhow,
        &context.aliases,
    )
    .unwrap()
}

#[test]
fn actual_parent_aliases_and_named_result_rhs_reach_child() {
    let report = analyze(
        "use anyhow::Result; pub struct Response; pub type CommandResult = Result<Response>; mod run;",
        "use super::*; pub fn execute()->CommandResult {loop{}} pub fn load()->Result<Response>{loop{}}",
    );
    assert!(
        report.functions.iter().all(|f| f.returns_closed_result),
        "{:?}",
        report.functions
    );
    assert!(
        report
            .functions
            .iter()
            .all(|f| f.resolved_error.as_deref() == Some("anyhow::Error"))
    );
}

#[test]
fn imported_alias_renaming_preserves_actual_shape_and_shadow_refuses() {
    let report = analyze(
        "use anyhow::Result; pub struct Response; pub type CommandResult = Result<Response>;",
        "use super::CommandResult as RunResult; fn run()->RunResult {loop{}}",
    );
    assert!(report.functions[0].returns_closed_result);
    let report = analyze(
        "use anyhow::Result;",
        "use super::*; struct Result<T>(T); fn run()->Result<Response>{loop{}}",
    );
    assert!(!report.functions[0].returns_closed_result);
}

#[test]
fn alias_hiding_dynamic_json_or_tuple_does_not_close() {
    for alias in [
        "anyhow::Result<serde_json::Value>",
        "(serde_json::Value,i32)",
    ] {
        let report = analyze(
            &format!("pub type CommandResult={alias};"),
            "use super::*; fn run()->CommandResult {loop{}}",
        );
        assert!(!report.functions[0].returns_closed_result);
    }
}

#[test]
fn exact_reexports_propagate_named_results_through_intermediate_modules() {
    let files = BTreeMap::from([
        (
            "src/cli/response.rs".into(),
            b"struct Response; type CommandResult=Result<Response,anyhow::Error>;".to_vec(),
        ),
        (
            "src/cli/handlers/mod.rs".into(),
            b"pub(super) use super::response::CommandResult; mod run;".to_vec(),
        ),
        (
            "src/cli/handlers/run.rs".into(),
            b"use super::CommandResult; fn execute()->CommandResult {loop{}}".to_vec(),
        ),
    ]);
    let contexts = contexts(&files);
    let context = &contexts["src/cli/handlers/run.rs"];
    let report = syntax::analyze_with_full_context(
        "src/cli/handlers/run.rs",
        std::str::from_utf8(&files["src/cli/handlers/run.rs"]).unwrap(),
        false,
        &context.anyhow,
        &context.aliases,
    )
    .unwrap();
    assert!(
        report.functions[0].returns_closed_result,
        "{:?}",
        report.functions
    );
}
