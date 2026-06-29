use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn toml_string_field_reads_quoted_multiline_and_missing_values() {
    let text = "name = \"Agent\"\ndescription = \"\"\"Long\"\"\"\n";
    assert_eq!(
        super::toml_string_field(text, "name").as_deref(),
        Some("Agent")
    );
    assert_eq!(
        super::toml_string_field(text, "description").as_deref(),
        Some("multiline-string")
    );
    assert!(super::toml_string_field(text, "missing").is_none());
}

#[test]
fn plugin_policy_cache_reuses_manifest_failure_sets() {
    let root = crate::self_tests::boundaries::support::temp_root("plugin-policy-cache");
    std::fs::create_dir_all(&root).expect("plugin policy cache root");
    let bundle = json!({"plugin_manifest":{"resources":["missing.txt"]}});
    let mut cache = BTreeMap::new();
    let mut first = Vec::new();
    super::check_plugin_with_cache(&bundle, &root, &mut first, &mut cache);
    assert_eq!(cache.len(), 1);
    let mut second = Vec::new();
    super::check_plugin_with_cache(&bundle, &root, &mut second, &mut cache);
    assert_eq!(failure_triples(&first), failure_triples(&second));
    std::fs::remove_dir_all(root).expect("cleanup plugin policy cache");
}

fn failure_triples(failures: &[crate::audit::contract::Failure]) -> Vec<(&str, &str, &str)> {
    failures
        .iter()
        .map(|failure| {
            (
                failure.check_id.as_str(),
                failure.error.as_str(),
                failure.detail.as_str(),
            )
        })
        .collect()
}
