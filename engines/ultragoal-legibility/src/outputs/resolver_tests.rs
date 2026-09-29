use super::*;
#[test]
fn real_pub_reexport_reaches_actual_source_function() {
    let source: BTreeMap<String, Vec<u8>> = BTreeMap::from([
        (
            "src/lib.rs".into(),
            b"mod internal; pub use internal::helper;".to_vec(),
        ),
        (
            "src/internal.rs".into(),
            b"pub fn helper(){std::fs::read(\"input\");}".to_vec(),
        ),
        ("src/ui.rs".into(), b"fn make(){crate::helper();}".to_vec()),
    ]);
    let reports = source
        .iter()
        .map(|(p, s)| {
            (
                p.clone(),
                crate::syntax::analyze(p, std::str::from_utf8(s).unwrap()).unwrap(),
            )
        })
        .collect();
    let aliases = crate::source_context::aliases(&source, &reports);
    let trace = graph::trace_with_aliases("src/ui.rs", "make", &reports, &aliases);
    assert!(
        trace
            .sites
            .contains(&("src/internal.rs".into(), "helper".into()))
    );
    assert!(!trace.unknown.iter().any(|u| u.call == "crate::helper"));
    assert!(trace.unknown.iter().any(|u| u.call == "std::fs::read"));
}
#[test]
fn exclusive_cfg_functions_check_both_bodies_without_guessing() {
    let source = "#[cfg(target_os=\"macos\")] fn same(){String::new();} #[cfg(not(target_os=\"macos\"))] fn same(){std::fs::read(\"input\");} fn start(){same();}";
    let reports = BTreeMap::from([(
        "src/lib.rs".into(),
        crate::syntax::analyze("src/lib.rs", source).unwrap(),
    )]);
    let trace = graph::trace("src/lib.rs", "start", &reports);
    assert!(
        !trace.unknown.iter().any(|u| u.reason.contains("ambiguous")),
        "{:?}",
        trace.unknown
    );
    assert!(trace.unknown.iter().any(|u| u.call == "std::fs::read"));
    let duplicate =
        crate::syntax::analyze("src/lib.rs", "fn same(){} fn same(){} fn start(){same();}")
            .unwrap();
    assert!(!crate::conditional::exclusive(
        &duplicate
            .functions
            .iter()
            .filter(|f| f.name == "same")
            .collect::<Vec<_>>()
    ));
}
