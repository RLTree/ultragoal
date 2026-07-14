#[test]
fn source_text_scanner_routes_raw_and_output_authority_failures() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("authority-source-scanner");
    std::fs::create_dir_all(root.join("validator/src/domain")).expect("source dir");
    std::fs::write(
        root.join("validator/src/domain/claim_core.rs"),
        "#![allow(dead_code)]\nuse serde_json::Value;\npub(crate) fn decide(value: Value) -> Value { let receipt = root.join(&command.receipt); crate::json_boundary::write_json(&receipt, &value).unwrap(); value }\n",
    )
    .expect("raw source");
    let failures = crate::audit::law::authority_surfaces::source_text_failures_for_test(&root);
    assert!(
        failures.iter().any(|(check, failure)| {
            check == "governed-source-inventory"
                && failure.contains("plugin_self_law_forbidden_lint_allowance")
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(check, failure)| {
            check == "typed-records-over-prose"
                && failure.contains("raw_downstream_authority_unclassified")
        }),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|(check, failure)| {
            check == "total-authority-types-impossible-state-elimination"
                && failure.contains("claim_artifact_output_without_typed_authority")
        }),
        "{failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup source scanner");
}

#[test]
fn source_text_scanner_excludes_only_test_gated_modules() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root(
        "authority-source-scanner-test-gates",
    );
    std::fs::create_dir_all(root.join("validator/src/cli/observe/command_roundtrip"))
        .expect("observe source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/package/inventory"))
        .expect("package source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/current_state"))
        .expect("current state source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/final_packet/proof"))
        .expect("final packet source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/live_loop/observation"))
        .expect("live-loop observation source dir");
    std::fs::create_dir_all(root.join("validator/src/cli/live_loop/observation/status"))
        .expect("live-loop observation status source dir");

    let raw_output_write = "use serde_json::Value;\n\
fn raw_write(root: &std::path::Path, value: &Value) {\n\
    let receipt_path = std::path::PathBuf::from(\"/tmp/claim-output.json\");\n\
    crate::json_boundary::write_json(&receipt_path, value).unwrap();\n\
}\n";
    std::fs::write(
        root.join("validator/src/cli/observe/command_roundtrip/process.rs"),
        format!("pub(crate) fn run() {{}}\n#[cfg(test)]\nmod tests {{\n{raw_output_write}\n}}\n"),
    )
    .expect("inline test module");
    std::fs::write(
        root.join("validator/src/cli/package/inventory/mod.rs"),
        "#[cfg(test)]\nmod command_paths;\n",
    )
    .expect("test sibling parent");
    std::fs::write(
        root.join("validator/src/cli/package/inventory/command_paths.rs"),
        raw_output_write,
    )
    .expect("test sibling module");
    std::fs::write(
        root.join("validator/src/cli/current_state/mod.rs"),
        "mod command_paths;\n",
    )
    .expect("production sibling parent");
    std::fs::write(
        root.join("validator/src/cli/current_state/command_paths.rs"),
        raw_output_write,
    )
    .expect("production sibling module");
    std::fs::write(
        root.join("validator/src/cli/final_packet/proof/observability.rs"),
        "use serde_json::Value;\n\
#[cfg(test)]\n\
mod tests;\n\
pub(crate) fn failures(value: &Value) -> Vec<String> {\n\
    let mut out = Vec::new();\n\
    if value.get(\"status\").and_then(Value::as_str).is_none() {\n\
        out.push(\"status_missing\".to_string());\n\
    }\n\
    out\n\
}\n",
    )
    .expect("declaration test module with production parser after it");
    std::fs::write(
        root.join("validator/src/cli/live_loop/observation/mod.rs"),
        "#[cfg(test)]\n#[path = \"status/success.rs\"]\nmod success;\n#[cfg(test)]\nmod status;\n",
    )
    .expect("path test module parent");
    std::fs::write(
        root.join("validator/src/cli/live_loop/observation/status/success.rs"),
        "use serde_json::json;\n\
#[test]\n\
fn verifies_status_projection() {\n\
    let value = json!({\"status\":\"pass\"});\n\
    assert_eq!(value[\"status\"], \"pass\");\n\
}\n",
    )
    .expect("path test sibling");
    std::fs::write(
        root.join("validator/src/cli/live_loop/observation/status.rs"),
        "#[path = \"status_nested.rs\"]\nmod status_nested;\n",
    )
    .expect("nested path test parent");
    std::fs::write(
        root.join("validator/src/cli/live_loop/observation/status_nested.rs"),
        "use serde_json::json;\n\
#[test]\n\
fn verifies_nested_status_projection() {\n\
    let value = json!({\"status\":\"pass\"});\n\
    assert_eq!(value[\"status\"], \"pass\");\n\
}\n",
    )
    .expect("nested path test sibling");

    let failures = crate::audit::law::authority_surfaces::source_text_failures_for_test(&root);
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/observe/command_roundtrip/process.rs")),
        "inline cfg(test) modules must not be production authority failures: {failures:?}"
    );
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/package/inventory/command_paths.rs")),
        "cfg(test) sibling modules must not be production authority failures: {failures:?}"
    );
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/live_loop/observation/status/success.rs")),
        "cfg(test) path sibling modules must not be production authority surfaces: {failures:?}"
    );
    assert!(
        failures.iter().all(|(_, failure)| !failure
            .contains("validator/src/cli/live_loop/observation/status_nested.rs")),
        "nested cfg(test) path sibling modules must not be production authority surfaces: {failures:?}"
    );
    assert!(
        failures.iter().any(
            |(_, failure)| failure.contains("validator/src/cli/current_state/command_paths.rs")
        ),
        "non-test-gated sibling modules must remain production authority surfaces: {failures:?}"
    );
    assert!(
        failures.iter().any(|(check, failure)| {
            check == "typed-records-over-prose"
                && failure.contains(
                    "raw_authority_unregistered:path=validator/src/cli/final_packet/proof/observability.rs;symbol=failures",
                )
        }),
        "production parser code after cfg(test) module declarations must remain scanned and require an exact typed-boundary registry row: {failures:?}"
    );
    std::fs::remove_dir_all(root).expect("cleanup source scanner test gates");
}
