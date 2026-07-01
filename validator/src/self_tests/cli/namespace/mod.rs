use serde_json::{Value, json};
use std::path::{Path, PathBuf};

mod edges;

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

fn write_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, body).expect("write file");
}

fn support_files(root: &Path) -> Vec<String> {
    write_file(
        &root.join("validator/src/domain/leaf.rs"),
        "pub fn leaf() {}\n",
    );
    write_json(
        &root.join("docs/namespace-class-registry.json"),
        &json!({"schema":"harness-ultragoal.namespace-class-registry.v1","classes":[
            class("repo-owned-rust-source","repo_source",json!(["validator/src/**/*.rs"]),"validator/src/domain/leaf.rs"),
            class("documentation-surfaces","documentation",json!(["docs/**"]),"docs/namespace-class-registry.json"),
            class("template-surfaces","template",json!(["templates/**"]),"templates/RED_FIXTURES.json"),
            class("public-distribution-surfaces","public_distribution",json!(["plugin-manifest-draft.json"]),"plugin-manifest-draft.json")
        ]}),
    );
    write_json(
        &root.join("templates/agent-standards/enforcement.json"),
        &json!({"rows":[{"id":"namespace-progressive-disclosure"}]}),
    );
    write_json(
        &root.join("docs/foundational-law-traceability.json"),
        &json!({"entries":[{"obligation_id":"namespace-progressive-disclosure"}]}),
    );
    write_json(
        &root.join("templates/RED_FIXTURES.json"),
        &json!([{"id":"namespace-red","expected_failure":{"check_id":"namespace-progressive-disclosure"}}]),
    );
    vec![
        "validator/src/domain/leaf.rs".to_string(),
        "docs/namespace-class-registry.json".to_string(),
        "templates/agent-standards/enforcement.json".to_string(),
        "docs/foundational-law-traceability.json".to_string(),
        "templates/RED_FIXTURES.json".to_string(),
        "plugin-manifest-draft.json".to_string(),
    ]
}

fn class(id: &str, kind: &str, globs: Value, authority_path: &str) -> Value {
    json!({
        "id": id,
        "kind": kind,
        "description": "test namespace class",
        "authority": "test_authority",
        "authority_path": authority_path,
        "surface_globs": globs,
        "maximal_factoring_required": true,
        "waiver_allowed": false,
        "claim_ceiling_impact": "classifies_surface_without_raising_claim_ceiling"
    })
}

fn package_root(label: &str, extra_files: &[(&str, &str)]) -> PathBuf {
    let root = crate::self_tests::boundaries::support::temp_root(label);
    let mut resources = support_files(&root);
    for (rel, body) in extra_files {
        write_file(&root.join(rel), body);
        resources.push((*rel).to_string());
    }
    resources.sort();
    resources.dedup();
    write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources": resources}),
    );
    root
}

fn args(root: PathBuf, raw: &[&str]) -> crate::Args {
    crate::Args {
        root,
        command: crate::parse_command(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("parse command"),
    }
}

#[test]
fn namespace_parser_routes_to_dedicated_command() {
    let raw = ["namespace", "check", "--strict", "--jobs", "2"];
    let command =
        crate::cli::namespace::parse(&raw.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .expect("namespace parse")
            .expect("namespace command");
    assert_eq!(command.jobs, Some(2));
    assert_eq!(
        command.receipt,
        PathBuf::from("validation_artifacts/observability/namespace-check.json")
    );
    let missing = ["namespace", "check"];
    let err = crate::parse_command(&missing.iter().map(|s| s.to_string()).collect::<Vec<_>>())
        .expect_err("strict flag required");
    assert!(err.contains("requires --strict"), "{err}");
    let err = crate::parse_command(
        &["namespace", "check", "--strict", "--receipt"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("missing receipt value");
    assert!(err.contains("missing value for --receipt"), "{err}");
    let err = crate::parse_command(
        &["namespace", "check", "--strict", "--bogus"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>(),
    )
    .expect_err("unknown argument");
    assert!(err.contains("unknown namespace check argument"), "{err}");
}

#[test]
fn namespace_command_writes_pass_observability_receipt() {
    let root = package_root("namespace-pass", &[]);
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["namespace", "check", "--strict", "--jobs", "2"],
    ))
    .expect("namespace pass");
    assert_eq!(code, 0);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/namespace-check.json"),
    )
    .expect("receipt");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    assert_eq!(receipt["status"], "pass");
    assert_eq!(receipt["candidate_digest"], candidate);
    assert_eq!(receipt["check_id"], "namespace-check-observability-binding");
    assert_eq!(receipt["claim_id"], "namespace_check");
    assert_eq!(receipt["event"]["operation"], "namespace.check");
    assert_eq!(receipt["event"]["failure_class"], "none");
    assert!(receipt["event"]["duration_ms"].as_u64().unwrap() > 0);
    assert_eq!(receipt["event"]["task_count"], 4);
    assert_eq!(receipt["event"]["queue_depth"], 4);
    assert!(
        receipt["trace"]["child_spans"]
            .as_array()
            .expect("child spans")
            .iter()
            .all(|span| span["parent_span_id"] == receipt["trace"]["span_id"])
    );
    let stdout = crate::cli::namespace::stdout_contract_for_test(&receipt);
    assert_eq!(stdout.len(), 1);
    assert!(stdout[0].contains("ultragoal-namespace-check pass"));
    assert!(stdout[0].contains("supported_claims=namespace_check"));
    assert!(stdout[0].contains("unsupported_claims="));
    let absolute_receipt = root.join("target/absolute-namespace-check.json");
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &[
            "namespace",
            "check",
            "--strict",
            "--receipt",
            absolute_receipt.to_str().expect("utf8 path"),
        ],
    ))
    .expect("absolute namespace receipt");
    assert_eq!(code, 0);
    assert!(absolute_receipt.is_file());
    std::fs::remove_dir_all(root).expect("cleanup namespace pass");
}

#[test]
fn namespace_command_fails_bad_namespace_with_repair_fields() {
    let root = package_root("namespace-fail", &[("docs/helpers/readme.md", "bad")]);
    let code = crate::command_run::run_with_exit_code(args(
        root.clone(),
        &["namespace", "check", "--strict"],
    ))
    .expect("namespace fail receipt");
    assert_eq!(code, 1);
    let receipt = crate::json_boundary::read_json(
        &root.join("validation_artifacts/observability/namespace-check.json"),
    )
    .expect("receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["event"]["failure_class"], "namespace_law_failure");
    assert_eq!(receipt["where_failed"], "namespace.check");
    assert!(
        receipt["why_failed"]
            .as_str()
            .expect("why")
            .contains("namespace_junk_drawer_path:docs/helpers/readme.md")
    );
    assert!(
        receipt["next_repair"]
            .as_str()
            .expect("next repair")
            .contains("semantic namespace")
    );
    assert!(
        receipt["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );
    let stdout = crate::cli::namespace::stdout_contract_for_test(&receipt);
    assert_eq!(stdout.len(), 2);
    assert!(stdout[0].contains("ultragoal-namespace-check fail"));
    assert!(stdout[0].contains("supported_claims=none"));
    assert!(stdout[0].contains("unsupported_claims="));
    assert!(stdout[1].contains("failed_check=namespace-check-observability-binding"));
    assert!(stdout[1].contains("next_repair="));
    std::fs::remove_dir_all(root).expect("cleanup namespace fail");
}
