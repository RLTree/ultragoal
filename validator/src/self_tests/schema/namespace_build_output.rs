use serde_json::json;
use std::path::Path;

#[test]
fn namespace_law_excludes_generated_build_output_from_source_orphan_authority() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "namespace-law-generated-build-output",
    );
    write_text(&root.join("validator/target/test-spool/events.jsonl"), "{}");
    write_text(&root.join("target/cargo-cache/build.log"), "build output");
    write_text(&root.join("src/orphan.rs"), "mod orphan;");

    let failures = crate::audit::namespace::law::value_failures(&root, &json!({"resources":[]}));

    assert!(
        failures.iter().all(|item| {
            !item.contains("validator/target/") && !item.contains("target/cargo-cache")
        }),
        "generated build output must not become namespace source authority: {failures:?}"
    );
    assert!(
        failures
            .iter()
            .any(|item| item.contains("namespace_orphan_repo_file:src/orphan.rs")),
        "real repo source orphan must still fail: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup generated build output");
}

fn write_text(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, text).expect("write text");
}
