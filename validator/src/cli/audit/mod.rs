use std::path::{Path, PathBuf};

#[cfg(test)]
mod edge_tests;
mod observability;

pub(crate) struct RunArgs {
    pub(crate) root: PathBuf,
    pub(crate) receipt: PathBuf,
    pub(crate) red_report: Option<PathBuf>,
    pub(crate) target_repo: Option<PathBuf>,
    pub(crate) mode: String,
    pub(crate) require_observability: bool,
    pub(crate) require_product_cohesion: bool,
    pub(crate) jobs: Option<usize>,
}

pub(crate) fn run(args: RunArgs) -> Result<i32, String> {
    let root = args.root.clone();
    let receipt = args.receipt.clone();
    let target_repo = args.target_repo.clone();
    let red_report = args.red_report.clone();
    let options = crate::audit::AuditOptions {
        root: args.root,
        receipt: args.receipt,
        red_report: args.red_report,
        target_repo: args.target_repo,
        mode: args.mode,
        require_observability: args.require_observability,
        require_product_cohesion: args.require_product_cohesion,
        jobs: args.jobs,
        command_text: command_text(&root),
    };
    let result = crate::audit::run(options);
    match result {
        Ok(code) => {
            observability::write_all(
                &root,
                &receipt,
                red_report.as_deref(),
                code,
                target_repo.is_some(),
                None,
            )?;
            Ok(code)
        }
        Err(err) => {
            let _ = observability::write_all(
                &root,
                &receipt,
                red_report.as_deref(),
                1,
                target_repo.is_some(),
                Some(&err),
            );
            Err(err)
        }
    }
}

fn command_text(root: &Path) -> String {
    let raw = std::env::args().collect::<Vec<_>>().join(" ");
    raw.replace(&root.to_string_lossy().to_string(), ".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn source_audit_observability_receipt_blocks_claims_on_failed_audit() {
        let root = crate::self_tests::boundaries::support::temp_root("source-audit-observe");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join("owned.txt"), "owned").expect("owned");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["owned.txt"]}),
        )
        .expect("manifest");
        write_minimal_law_surfaces(&root);
        let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
        fs::create_dir_all(receipt.parent().unwrap()).expect("audit dir");
        crate::json_boundary::write_json(
            &receipt,
            &json!({
                "status":"fail",
                "checks": {
                    "coverage-receipt": {"status":"fail"},
                    "schema-valid": {"status":"pass"}
                }
            }),
        )
        .expect("audit receipt");
        observability::write_all(&root, &receipt, None, 1, false, None).expect("observability");
        let value = crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT))
            .expect("obs");
        assert_eq!(value["status"], "fail");
        assert_eq!(value["operation"], "source.audit");
        assert_eq!(value["claim_id"], "source_audit");
        assert!(
            value["why_failed"]
                .as_str()
                .unwrap_or_default()
                .contains("coverage-receipt")
        );
        assert!(
            value["blocked_claims"]
                .as_array()
                .expect("blocked")
                .iter()
                .any(|item| item.as_str() == Some("update_goal_eligibility"))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn source_audit_run_writes_observability_through_cli_boundary() {
        let root = crate::self_tests::boundaries::support::temp_root("source-audit-run-observe");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join("owned.txt"), "owned").expect("owned");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["owned.txt"]}),
        )
        .expect("manifest");
        write_minimal_law_surfaces(&root);
        let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
        let code = run(RunArgs {
            root: root.clone(),
            receipt: receipt.clone(),
            red_report: None,
            target_repo: None,
            mode: "source".to_string(),
            require_observability: false,
            require_product_cohesion: false,
            jobs: Some(2),
        })
        .expect("audit run");
        assert_ne!(code, 0);
        let audit_receipt = crate::json_boundary::read_json(&receipt).expect("audit receipt");
        assert_eq!(
            audit_receipt["scheduler_execution"][0]["task_class"],
            "pure_read_parallel"
        );
        assert!(
            audit_receipt["scheduler_execution"][0]["worker_count"]
                .as_u64()
                .expect("worker count")
                <= 2
        );
        assert_eq!(
            audit_receipt["scheduler_execution"][0]["claim_impact"],
            "supports_source_local_scheduler_timing_only_not_readiness"
        );
        let obs = crate::json_boundary::read_json(&root.join(observability::SOURCE_RECEIPT))
            .expect("source audit obs");
        assert_eq!(obs["status"], "fail");
        assert_eq!(obs["operation"], "source.audit");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn audit_observability_records_target_pass_and_red_report_failures() {
        let root = crate::self_tests::boundaries::support::temp_root("audit-observe-pass-red");
        fs::create_dir_all(&root).expect("root");
        fs::write(root.join("owned.txt"), "owned").expect("owned");
        crate::json_boundary::write_json(
            &root.join("plugin-manifest-draft.json"),
            &json!({"resources":["owned.txt"]}),
        )
        .expect("manifest");
        let receipt = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
        crate::json_boundary::write_json(&receipt, &json!({"status":"pass","checks":{}}))
            .expect("receipt");
        let red = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
        crate::json_boundary::write_json(
            &red,
            &json!({"status":"fail","red_fixtures":{"bad":{"status":"fail"}}}),
        )
        .expect("red");
        observability::write_all(&root, &receipt, Some(&red), 0, true, None).expect("observe");
        let target = crate::json_boundary::read_json(
            &root.join("validation_artifacts/observability/target-repo-audit.json"),
        )
        .expect("target obs");
        assert_eq!(target["status"], "pass");
        assert_eq!(target["operation"], "target-repo.audit");
        assert!(
            target["supported_claims"]
                .as_array()
                .unwrap()
                .iter()
                .any(|claim| claim.as_str() == Some("source_local_audit_checks"))
        );
        let red_obs = crate::json_boundary::read_json(
            &root.join("validation_artifacts/observability/red-fixture-report.json"),
        )
        .expect("red obs");
        assert_eq!(red_obs["status"], "fail");
        assert!(
            red_obs["why_failed"]
                .as_str()
                .unwrap_or_default()
                .contains("bad")
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    fn write_minimal_law_surfaces(root: &std::path::Path) {
        fs::create_dir_all(root.join("examples/generated")).expect("generated dir");
        crate::json_boundary::write_json(&root.join("schemas/schema-catalog.json"), &json!([]))
            .expect("schema catalog");
        crate::json_boundary::write_json(
            &root.join("docs/mandatory-law-surfaces.json"),
            &json!({"laws":[]}),
        )
        .expect("mandatory laws");
        crate::json_boundary::write_json(
            &root.join("docs/source-obligation-matrix.json"),
            &json!({"obligations":[]}),
        )
        .expect("obligations");
        crate::json_boundary::write_json(
            &root.join("templates/agent-standards/enforcement.json"),
            &json!({"rows":[]}),
        )
        .expect("standards");
        crate::json_boundary::write_json(&root.join("templates/RED_FIXTURES.json"), &json!([]))
            .expect("red fixtures");
    }
}
