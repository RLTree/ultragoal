use super::*;
#[test]
fn real_glob_exports_resolve_without_fabricating_prelude_members() {
    let files = BTreeMap::from([
        (
            "src/parent/mod.rs".into(),
            b"use std::{fs,path::Path}; pub fn helper(){} mod child;".to_vec(),
        ),
        (
            "src/parent/child.rs".into(),
            b"use super::*; fn run(){Some(Vec::new()); fs::read(\"input\");helper();}".to_vec(),
        ),
    ]);
    let contexts = bindings::contexts(&files);
    let source = std::str::from_utf8(&files["src/parent/child.rs"]).unwrap();
    let report = syntax::analyze_with_bindings(
        "src/parent/child.rs",
        source,
        false,
        &[],
        &[],
        &contexts["src/parent/child.rs"],
    )
    .unwrap();
    let calls = &report.functions[0].direct_calls;
    assert!(
        calls.contains(&"std::option::Option::Some".into()),
        "{calls:?}"
    );
    assert!(calls.contains(&"std::vec::Vec::new".into()));
    assert!(calls.contains(&"std::fs::read".into()));
    assert!(calls.contains(&"crate::parent::helper".into()));
    assert!(
        !calls
            .iter()
            .any(|c| c == "super::Some" || c == "super::Vec::new")
    );
}
#[test]
fn actual_glob_import_can_shadow_a_prelude_constructor() {
    let files = BTreeMap::from([
        (
            "src/parent/mod.rs".into(),
            b"pub fn Some(){} mod child;".to_vec(),
        ),
        (
            "src/parent/child.rs".into(),
            b"use super::*; fn run(){Some();}".to_vec(),
        ),
    ]);
    let contexts = bindings::contexts(&files);
    let report = syntax::analyze_with_bindings(
        "src/parent/child.rs",
        std::str::from_utf8(&files["src/parent/child.rs"]).unwrap(),
        false,
        &[],
        &[],
        &contexts["src/parent/child.rs"],
    )
    .unwrap();
    assert_eq!(
        report.functions[0].direct_calls,
        vec!["crate::parent::Some"]
    );
}

#[test]
fn grouped_module_self_import_does_not_capture_value_receiver() {
    let source = "use std::fs::{self,File}; struct Owned{path:u8} impl Owned{fn value(&self)->u8{self.path}}";
    let files = BTreeMap::from([("src/owned.rs".into(), source.as_bytes().to_vec())]);
    let contexts = bindings::contexts(&files);
    assert_eq!(contexts["src/owned.rs"]["fs"], vec!["std::fs"]);
    assert!(!contexts["src/owned.rs"].contains_key("self"));
    let report = syntax::analyze_with_bindings(
        "src/owned.rs",
        source,
        false,
        &[],
        &[],
        &contexts["src/owned.rs"],
    )
    .unwrap();
    assert!(!report.authorities.iter().any(|a| a.kind == "filesystem"));
}
