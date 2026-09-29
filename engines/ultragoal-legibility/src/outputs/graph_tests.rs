use super::graph::trace;
use crate::syntax;
use std::collections::BTreeMap;

fn reports(sources: &[(&str, &str)]) -> BTreeMap<String, syntax::Report> {
    sources
        .iter()
        .map(|(path, source)| (path.to_string(), syntax::analyze(path, source).unwrap()))
        .collect()
}

#[test]
fn walks_local_helper_and_stops_cycles() {
    let reports = reports(&[(
        "src/lib.rs",
        "fn emit() { helper(); } fn helper() { emit(); }",
    )]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert!(result.unknown.is_empty(), "{:?}", result.unknown);
    assert_eq!(
        result.sites,
        vec![
            ("src/lib.rs".into(), "emit".into()),
            ("src/lib.rs".into(), "helper".into())
        ]
    );
}

#[test]
fn resolves_crate_super_and_import_aliases() {
    let reports = reports(&[
        ("src/lib.rs", "fn finish() {}"),
        (
            "src/output/mod.rs",
            "use crate::output::detail::helper as run; fn emit() { run(); }",
        ),
        (
            "src/output/detail.rs",
            "fn helper() { super::sibling::other(); crate::finish(); }",
        ),
        ("src/output/sibling.rs", "fn other() {}"),
    ]);
    let result = trace("src/output/mod.rs", "emit", &reports);
    assert!(result.unknown.is_empty(), "{:?}", result.unknown);
    assert_eq!(result.sites.len(), 4);
}

#[test]
fn process_helper_remains_reachable_and_effect_is_unresolved() {
    let reports = reports(&[(
        "src/lib.rs",
        "fn emit() { helper(); } fn helper() { std::process::Command::new(\"echo\"); }",
    )]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert!(
        result
            .sites
            .contains(&("src/lib.rs".into(), "helper".into()))
    );
    assert!(
        result
            .unknown
            .iter()
            .any(|unknown| unknown.call == "std::process::Command::new")
    );
}

#[test]
fn ambiguous_internal_targets_and_unresolved_calls_fail_closed() {
    let reports = reports(&[
        (
            "src/lib.rs",
            "fn emit() { crate::helper::run(); missing(); }",
        ),
        ("src/helper.rs", "fn run() {}"),
        ("src/helper/mod.rs", "fn run() {}"),
    ]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert!(
        result
            .unknown
            .iter()
            .any(|unknown| unknown.reason == "ambiguous_call"
                && unknown.call == "crate::helper::run")
    );
    assert!(
        result
            .unknown
            .iter()
            .any(|unknown| unknown.reason == "unresolved_call" && unknown.call == "missing")
    );
}

#[test]
fn serializers_are_known_external_calls_without_blanket_dependency_approval() {
    let reports = reports(&[(
        "src/lib.rs",
        "fn emit() { serde_json::to_string(&1); serde_json::to_vec_pretty(&1); other::encode(&1); }",
    )]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert_eq!(result.unknown.len(), 1);
    assert!(result.unknown[0].call == "other::encode");
}

#[test]
fn nested_tool_crates_do_not_supply_main_crate_calls() {
    let reports = reports(&[
        ("src/lib.rs", "fn emit() { crate::helper(); }"),
        ("tools/legibility/src/lib.rs", "fn helper() {}"),
    ]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert_eq!(result.sites.len(), 1);
    assert!(
        result
            .unknown
            .iter()
            .any(|unknown| unknown.call == "crate::helper")
    );
}

#[test]
fn inline_module_self_paths_resolve_with_lexical_scope() {
    let reports = reports(&[(
        "src/lib.rs",
        "mod inner { fn emit() { self::helper(); } fn helper() {} }",
    )]);
    let result = trace("src/lib.rs", "inner::emit", &reports);
    assert!(result.unknown.is_empty(), "{:?}", result.unknown);
    assert_eq!(result.sites.len(), 2);
}

#[test]
fn pure_name_shadowed_by_local_function_is_traversed() {
    let reports = reports(&[(
        "src/lib.rs",
        "fn emit() { serde_json::to_string(); } mod serde_json { fn to_string() { mystery(); } }",
    )]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert_eq!(result.sites.len(), 2);
    assert!(
        result
            .unknown
            .iter()
            .any(|unknown| unknown.call == "mystery")
    );
}

#[test]
fn unresolved_local_serializer_shadow_does_not_receive_external_purity() {
    let reports = reports(&[(
        "src/lib.rs",
        "fn emit() { serde_json::to_string(); } mod serde_json { pub use unknown::to_string; }",
    )]);
    let result = trace("src/lib.rs", "emit", &reports);
    assert_eq!(result.unknown.len(), 1);
    assert_eq!(result.unknown[0].call, "serde_json::to_string");
}
