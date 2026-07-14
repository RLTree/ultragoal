use crate::agent_discovery::{AgentDiscoveryErrorId, SourceAgentCatalog};
use crate::authority_fixtures::{CANDIDATE, SESSION, TempRepo};
use serde_json::json;
use std::fs;

fn source_error(label: &str, mutate: impl FnOnce(&mut serde_json::Value)) -> AgentDiscoveryErrorId {
    let repo = TempRepo::canonical();
    let path = repo.root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("manifest bytes")).expect("manifest JSON");
    mutate(&mut manifest);
    fs::write(
        path,
        serde_json::to_vec(&manifest).expect("mutated manifest"),
    )
    .expect("manifest write");
    let error = SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
        .expect_err(&format!("malformed {label} accepted"));
    error.id()
}

#[test]
fn canonical_manifest_parser_rejects_malformed_non_null_author_and_interface() {
    assert_eq!(
        source_error("author", |manifest| manifest["author"] = json!("Fixture")),
        AgentDiscoveryErrorId::InvalidSourceCatalog,
    );
    assert_eq!(
        source_error("interface", |manifest| {
            manifest["interface"] = json!("Fixture")
        }),
        AgentDiscoveryErrorId::InvalidSourceCatalog,
    );
}

#[test]
fn canonical_manifest_parser_rejects_unknown_nested_fields() {
    assert_eq!(
        source_error("author field", |manifest| {
            manifest["author"]["unadopted"] = json!(true)
        }),
        AgentDiscoveryErrorId::InvalidSourceCatalog,
    );
    assert_eq!(
        source_error("interface field", |manifest| {
            manifest["interface"]["unadopted"] = json!(true)
        }),
        AgentDiscoveryErrorId::InvalidSourceCatalog,
    );
}

#[test]
fn canonical_manifest_parser_accepts_the_supported_catalog_shape() {
    let repo = TempRepo::canonical();
    let source = SourceAgentCatalog::capture(&repo.root, CANDIDATE, SESSION)
        .expect("canonical source catalog");
    assert_eq!(source.plugin_version(), "0.0.11");
}
