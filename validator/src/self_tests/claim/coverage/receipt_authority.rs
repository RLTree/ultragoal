use crate::audit::contract::Failure;
use serde_json::{Value, json};
use std::path::Path;

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, serde_json::to_vec(value).expect("json")).expect("write json");
}

#[test]
fn coverage_receipt_authority_rejects_scalar_digest_and_report_substitutes() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-authority");
    std::fs::create_dir_all(root.join(".harness")).expect("harness");
    std::fs::create_dir_all(root.join("src")).expect("src");
    std::fs::write(root.join("src/lib.rs"), "fn main() {}\n").expect("src");
    std::fs::write(root.join(".harness/coverage-command"), "coverage\n").expect("command");
    write_json(
        &root.join(".harness/coverage-manifest.json"),
        &json!({
            "required_target_paths":["src"],
            "changed_file_coupling_policy":{"changed_files":["src/lib.rs"]},
            "source_discovery_rules":{"ignore":[]}
        }),
    );
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::coverage::receipt::authority::check(
        &json!({
            "claim_id":"COV",
            "tool_version":"",
            "workspace_root":"/wrong",
            "command_exit":1,
            "generated_by":"human",
            "percent_source":"prose",
            "source_tree_digest":crate::self_tests::boundaries::workspace_fixtures::sha('1'),
            "coverage_manifest_digest":crate::self_tests::boundaries::workspace_fixtures::sha('2'),
            "coverage_command_digest":crate::self_tests::boundaries::workspace_fixtures::sha('3'),
            "changed_files_digest":crate::self_tests::boundaries::workspace_fixtures::sha('4'),
            "machine_readable_report":{"path":"../escape.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('5')}
        }),
        &root,
        &mut out,
    );
    let got = errors(&out);
    for expected in [
        "coverage_receipt_tool_version_missing",
        "coverage_receipt_workspace_mismatch",
        "coverage_command_failed",
        "coverage_receipt_not_tool_generated",
        "coverage_percent_from_prose",
        "coverage_receipt_source_digest_mismatch",
        "coverage_receipt_manifest_digest_mismatch",
        "coverage_receipt_command_digest_mismatch",
        "coverage_receipt_changed_files_digest_mismatch",
        "coverage_report_missing",
    ] {
        assert!(got.contains(&expected), "{expected}: {got:?}");
    }
    out.clear();
    let report = root.join("report.json");
    std::fs::write(&report, "{").expect("malformed report");
    crate::claim_semantics::coverage::receipt::authority::check(
        &json!({
            "tool_version":"llvm-cov",
            "workspace_root":"/repo",
            "command_exit":0,
            "generated_by":"coverage-command",
            "percent_source":"machine_readable_report",
            "machine_readable_report":{"path":"report.json","digest":crate::digest::file(&report).expect("digest")}
        }),
        &root,
        &mut out,
    );
    assert!(errors(&out).contains(&"coverage_report_not_machine_readable"));
    std::fs::remove_dir_all(root).expect("cleanup coverage authority");
}

#[test]
fn coverage_receipt_authority_reports_missing_mismatch_and_template_fallbacks() {
    let root =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-report-branches");
    std::fs::create_dir_all(root.join(".harness")).expect("harness");
    std::fs::create_dir_all(root.join("src")).expect("src");
    std::fs::write(root.join("src/lib.rs"), "fn main() {}\n").expect("src");
    std::fs::write(root.join(".harness/coverage-command"), "coverage\n").expect("command");
    write_json(
        &root.join(".harness/coverage-manifest.json"),
        &json!({
            "required_target_paths":["src"],
            "changed_file_coupling_policy":{"changed_files":["src/lib.rs"]},
            "source_discovery_rules":{"ignore":[]}
        }),
    );

    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::coverage::receipt::authority::check(
        &json!({
            "claim_id":"COV",
            "tool_version":"llvm-cov",
            "workspace_root":"/repo",
            "command_exit":0,
            "generated_by":"coverage-command",
            "percent_source":"machine_readable_report",
            "machine_readable_report":{"path":"missing.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('0')}
        }),
        &root,
        &mut out,
    );
    assert!(errors(&out).contains(&"coverage_report_missing"));

    let report = root.join("report.json");
    write_json(&report, &json!({"data":[]}));
    out.clear();
    crate::claim_semantics::coverage::receipt::authority::check(
        &json!({
            "claim_id":"COV",
            "tool_version":"llvm-cov",
            "workspace_root":"/repo",
            "command_exit":0,
            "generated_by":"coverage-command",
            "percent_source":"machine_readable_report",
            "machine_readable_report":{"path":"report.json","digest":crate::self_tests::boundaries::workspace_fixtures::sha('1')}
        }),
        &root,
        &mut out,
    );
    assert!(errors(&out).contains(&"coverage_report_digest_mismatch"));
    std::fs::remove_dir_all(root).expect("cleanup coverage report");

    let fallback =
        crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-template-fallback");
    std::fs::create_dir_all(fallback.join("templates/.harness")).expect("templates harness");
    std::fs::create_dir_all(fallback.join("src")).expect("fallback src");
    std::fs::write(fallback.join("src/lib.rs"), "fn fallback() {}\n").expect("src");
    std::fs::write(
        fallback.join("templates/.harness/coverage-command"),
        "coverage\n",
    )
    .expect("fallback command");
    write_json(
        &fallback.join("templates/.harness/coverage-manifest.json"),
        &json!({
            "required_target_paths":["src"],
            "changed_file_coupling_policy":{"changed_files":["src/lib.rs"]},
            "source_discovery_rules":{"ignore":[]}
        }),
    );
    let report = fallback.join("report.json");
    write_json(&report, &json!({"data":[]}));
    let receipt = json!({
        "claim_id":"COV",
        "tool_version":"llvm-cov",
        "workspace_root":"/repo",
        "command_exit":0,
        "generated_by":"coverage-command",
        "percent_source":"machine_readable_report",
        "source_tree_digest":crate::claim_semantics::coverage::digests::source_tree_digest(
            &fallback,
            &crate::json_boundary::read_json(&fallback.join("templates/.harness/coverage-manifest.json")).expect("manifest")
        ).expect("source digest"),
        "coverage_manifest_digest":crate::digest::file(&fallback.join("templates/.harness/coverage-manifest.json")).expect("manifest digest"),
        "coverage_command_digest":crate::digest::file(&fallback.join("templates/.harness/coverage-command")).expect("command digest"),
        "changed_files_digest":crate::claim_semantics::coverage::digests::changed_files_digest(
            &fallback,
            &crate::json_boundary::read_json(&fallback.join("templates/.harness/coverage-manifest.json")).expect("manifest")
        ).expect("changed digest"),
        "machine_readable_report":{"path":"report.json","digest":crate::digest::file(&report).expect("report digest")}
    });
    out.clear();
    crate::claim_semantics::coverage::receipt::authority::check(&receipt, &fallback, &mut out);
    assert!(out.is_empty(), "{out:?}");
    std::fs::remove_dir_all(fallback).expect("cleanup fallback coverage");
}

#[test]
fn coverage_digests_include_direct_file_targets() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-direct-file");
    std::fs::create_dir_all(root.join("src")).expect("src");
    std::fs::write(root.join("src/lib.rs"), "fn direct() {}\n").expect("src file");
    let manifest = json!({
        "required_target_paths":["src/lib.rs"],
        "changed_file_coupling_policy":{"changed_files":["src/lib.rs"]},
        "source_discovery_rules":{"ignore":[]}
    });
    let source = crate::claim_semantics::coverage::digests::source_tree_digest(&root, &manifest)
        .expect("source digest");
    let changed = crate::claim_semantics::coverage::digests::changed_files_digest(&root, &manifest)
        .expect("changed digest");
    assert_ne!(source, crate::digest::ZERO);
    assert_eq!(source, changed);
    std::fs::remove_dir_all(root).expect("cleanup direct file coverage");
}

#[test]
fn coverage_receipt_exclusions_reject_owned_unreviewed_counted_and_unrationaled_rows() {
    let root = crate::self_tests::boundaries::workspace_fixtures::temp_root("coverage-exclusions");
    std::fs::create_dir_all(root.join("receipts")).expect("receipts");
    let receipt_path = root.join("receipts/coverage.json");
    write_json(
        &receipt_path,
        &json!({
            "claim_id":"COV",
            "coverage_percent":100.0,
            "line_coverage_percent":100.0,
            "claim_ceiling":"complete",
            "uncovered_records":[],
            "exclusions":[
                {"path":"validator/src/lib.rs","reviewed":false,"counts_as_covered":true,"rationale":""},
                {"path":"docs/background.md","reviewed":true,"counts_as_covered":false,"rationale":"non-code background"}
            ]
        }),
    );
    let mut out = Vec::<Failure>::new();
    crate::claim_semantics::coverage::policy::check(
        &json!({
            "id":"COV",
            "title":"coverage complete",
            "claim_ceiling_effect":"included",
            "evidence":[{
                "id":"coverage",
                "kind":"coverage_receipt",
                "surface":"coverage",
                "path":"receipts/coverage.json",
                "digest":crate::digest::file(&receipt_path).expect("coverage digest")
            }]
        }),
        &root,
        &mut out,
    );
    let got = errors(&out);
    for expected in [
        "coverage_repo_owned_code_excluded",
        "coverage_exclusion_unreviewed",
        "coverage_exclusion_counted_as_covered",
        "coverage_exclusion_missing_rationale",
    ] {
        assert!(got.contains(&expected), "{expected}: {out:?}");
    }
    std::fs::remove_dir_all(root).expect("cleanup coverage exclusions");
}
