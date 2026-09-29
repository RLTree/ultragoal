use super::*;

fn inspected(leaf: &str, wrapper: &str) -> Report {
    let sources = [
        (
            "src/codec.rs",
            "pub trait Domain: sealed::Seal + serde::de::DeserializeOwned {} mod sealed { pub trait Seal {} impl Seal for crate::wrapper::Payload {} } impl Domain for crate::wrapper::Payload {} fn decode<T:Domain>(raw:&str)->Result<T,serde_json::Error>{serde_json::from_str(raw)}",
        ),
        ("src/wrapper/mod.rs", "pub mod types; pub use types::*;"),
        ("src/wrapper/types.rs", wrapper),
        ("src/leaf.rs", leaf),
        (
            "src/unrelated.rs",
            "#[derive(serde::Deserialize)] struct Leaf { open: serde_json::Value }",
        ),
    ];
    let files = sources
        .iter()
        .map(|(p, s)| (p.to_string(), s.as_bytes().to_vec()))
        .collect();
    let mut reports = sources
        .iter()
        .map(|(p, s)| (p.to_string(), crate::syntax::analyze(p, s).unwrap()))
        .collect();
    assert!(enforce(&mut reports, &files).is_empty());
    reports.remove("src/codec.rs").unwrap()
}

const WRAPPER: &str = "use crate::leaf::Leaf as Child; #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] pub struct Payload { children: Vec<Child> }";

#[test]
fn cross_file_field_closure_uses_actual_import_not_same_named_record() {
    let closed = "#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] pub struct Leaf { valid: bool }";
    assert!(inspected(closed, WRAPPER).functions[0].returns_closed_result);
    let open = closed.replace("bool", "serde_json::Value");
    assert!(!inspected(&open, WRAPPER).functions[0].returns_closed_result);
}

#[test]
fn owned_record_recursion_terminates_and_does_not_hide_open_leaf() {
    let leaf = "use crate::wrapper::Payload; #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] pub struct Leaf { parent: Option<Box<Payload>>, valid: bool }";
    assert!(inspected(leaf, WRAPPER).functions[0].returns_closed_result);
    assert!(
        !inspected(
            &leaf.replace("valid: bool", "valid: serde_json::Value"),
            WRAPPER
        )
        .functions[0]
            .returns_closed_result
    );
}

#[test]
fn ambiguous_owned_imports_fail_closed() {
    let leaf = "#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] pub struct Leaf { valid: bool }";
    let ambiguous = format!("use crate::unrelated::Leaf as Child; {WRAPPER}");
    assert!(!inspected(leaf, &ambiguous).functions[0].returns_closed_result);
}
