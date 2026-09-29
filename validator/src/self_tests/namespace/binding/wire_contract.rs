use super::{contains, write_text};

const PATH: &str = "validator/src/state/adopted_claims/schema.rs";
const WIRE_KEY: &str = concat!("current_live_evidence_", "status");

#[test]
fn namespace_scanner_allows_the_declared_legacy_wire_binding() {
    let failures = scan(&format!(
        "struct Claim {{\n    #[serde(rename = \"{WIRE_KEY}\")]\n    live_evidence_verification: String,\n}}\n"
    ));
    assert!(
        !contains(
            &failures,
            "namespace_validator_source_product_opaque_goal_work_string"
        ),
        "{failures:?}"
    );
}

#[test]
fn namespace_scanner_rejects_the_legacy_key_on_wrong_fields_or_paths() {
    for (path, field) in [
        (PATH, "command_status"),
        (
            "validator/src/state/other_schema.rs",
            "live_evidence_verification",
        ),
    ] {
        let failures = scan_at(
            path,
            &format!(
                "struct Claim {{\n    #[serde(rename = \"{WIRE_KEY}\")]\n    {field}: String,\n}}\n"
            ),
        );
        assert!(
            contains(
                &failures,
                "namespace_validator_source_product_opaque_goal_work_string"
            ),
            "{path}: {failures:?}"
        );
    }
}

#[test]
fn namespace_scanner_rejects_nonsemantic_and_unrecognized_serde_forms() {
    for source in [
        format!(
            "struct Claim {{\n    #[serde(rename = \"{WIRE_KEY}\")]\n    production_proof: String,\n}}\n"
        ),
        format!("enum Claim {{\n    #[serde(rename = \"{WIRE_KEY}\")]\n    Value,\n}}\n"),
        format!(
            "#[serde(rename = \"{WIRE_KEY}\")]\nstruct Claim {{\n    live_evidence_verification: String,\n}}\n"
        ),
        format!(
            "struct Claim {{\n    #[cfg_attr(test, serde(rename = \"{WIRE_KEY}\"))]\n    live_evidence_verification: String,\n}}\n"
        ),
    ] {
        let failures = scan(&source);
        assert!(
            contains(
                &failures,
                "namespace_validator_source_product_opaque_goal_work_string"
            ),
            "{source}: {failures:?}"
        );
    }
}

fn scan(source: &str) -> Vec<String> {
    scan_at(PATH, source)
}

fn scan_at(path: &str, source: &str) -> Vec<String> {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("namespace-wire");
    write_text(&root.join(path), source);
    let failures =
        crate::audit::namespace::source::identifiers::failures(&root, &[path.to_owned()]);
    std::fs::remove_dir_all(root).expect("cleanup namespace wire");
    failures
}
