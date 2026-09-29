use super::*;
fn inspect(source: &str) -> Report {
    let path = "src/codec.rs";
    let files = BTreeMap::from([(path.into(), source.as_bytes().to_vec())]);
    let mut reports =
        BTreeMap::from([(path.into(), crate::syntax::analyze(path, source).unwrap())]);
    assert!(enforce(&mut reports, &files).is_empty());
    reports.remove(path).unwrap()
}
const SEALED: &str = "#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct Payload { valid: bool } pub trait Domain: serde::de::DeserializeOwned+sealed::Seal {} impl<T:serde::de::DeserializeOwned+sealed::Seal> Domain for T {} mod sealed {pub trait Seal {} impl Seal for super::Payload {}} fn decode<T:Domain>(raw:&str)->Result<T,serde_json::Error>{serde_json::from_str(raw)}";

fn reexported(module: &str) -> Report {
    let sources = [
        ("src/journal/mod.rs", module),
        ("src/journal/sealed.rs", "pub trait Seal {}"),
        (
            "src/journal/records.rs",
            "use super::sealed; #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct Payload { valid: bool } pub trait Domain: sealed::Seal + serde::de::DeserializeOwned {} impl sealed::Seal for Payload {} impl Domain for Payload {}",
        ),
        (
            "src/codec.rs",
            "use crate::journal::Record; fn decode<T: Record>(raw: &str)->Result<T, serde_json::Error>{serde_json::from_str(raw)}",
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

#[test]
fn explicit_reexport_and_external_private_module_resolve_to_owned_seal() {
    let report =
        reexported("pub(crate) mod sealed; pub mod records; pub use records::{Domain as Record};");
    assert!(report.functions[0].returns_closed_result);
    for source in [
        "pub mod sealed; pub mod records; pub use records::Domain as Record;",
        "pub(crate) mod sealed; pub mod records; pub use missing::Domain as Record;",
        "pub(crate) mod sealed; pub mod records; pub use records::Domain as Record; pub use other::Domain as Record;",
        "pub(crate) mod sealed; pub mod records; pub use Alias as Record; pub use Record as Alias;",
    ] {
        assert!(
            !reexported(source).functions[0].returns_closed_result,
            "{source}"
        );
    }
}
#[test]
fn finite_private_seal_allows_concrete_payload_and_relaxed_bound_refuses() {
    let report = inspect(SEALED);
    assert!(
        report
            .functions
            .iter()
            .find(|f| f.name == "decode")
            .unwrap()
            .returns_closed_result
    );
    let report = inspect(&SEALED.replace("T:Domain>", "T:serde::de::DeserializeOwned>"));
    assert!(
        !report
            .functions
            .iter()
            .find(|f| f.name == "decode")
            .unwrap()
            .returns_closed_result
    );
}
#[test]
fn public_blanket_or_raw_payload_implementations_do_not_establish_seal() {
    for invalid in [
        SEALED.replace("mod sealed", "pub mod sealed"),
        SEALED.replace("impl Seal for super::Payload {}", "impl<T> Seal for T {}"),
        SEALED.replace(
            "impl Seal for super::Payload {}",
            "impl Seal for serde_json::Value {}",
        ),
        format!("{SEALED} macro_rules! hidden {{()=>{{impl<T> sealed::Seal for T {{}}}}}}"),
    ] {
        assert!(
            !inspect(&invalid)
                .functions
                .iter()
                .find(|f| f.name == "decode")
                .unwrap()
                .returns_closed_result
        );
    }
}

#[test]
fn finite_named_seal_rejects_open_record_fields_and_unknown_field_admission() {
    for invalid in [
        SEALED.replace("valid: bool", "binding: serde_json::Value"),
        SEALED.replace("#[serde(deny_unknown_fields)]", ""),
        SEALED.replace("valid: bool", "calls: Vec<Call>")
            + " #[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] struct Call { arguments: serde_json::Value }",
    ] {
        let report = inspect(&invalid);
        assert!(
            !report
                .functions
                .iter()
                .find(|f| f.name == "decode")
                .unwrap()
                .returns_closed_result
        );
        assert!(
            report
                .limitations
                .iter()
                .any(|l| l.starts_with("generic_payload_bound_unverified:decode:"))
        );
    }
}
