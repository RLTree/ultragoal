use std::path::{Path, PathBuf};

mod observability;

pub(crate) struct RunArgs {
    pub(crate) root: PathBuf,
    pub(crate) receipt: PathBuf,
    pub(crate) red_report: Option<PathBuf>,
    pub(crate) target_repo: Option<PathBuf>,
    pub(crate) mode: String,
    pub(crate) require_observability: bool,
    pub(crate) require_product_cohesion: bool,
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
}
