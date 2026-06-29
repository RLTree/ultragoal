use serde_json::{Value, json};
use std::path::Path;

pub(crate) fn ref_for(root: &Path, current: &str) -> Value {
    let receipt = schema_shaped_receipt(current);
    super::support::write_json(
        &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
        &receipt,
    );
    json!({
        "path": "validation_artifacts/ultragoal-audit/validator-receipt.json",
        "digest": crate::digest::file(
            &root.join("validation_artifacts/ultragoal-audit/validator-receipt.json")
        )
        .expect("source audit digest"),
        "status": "pass"
    })
}

fn schema_shaped_receipt(current: &str) -> Value {
    let repo = crate::self_tests::boundaries::support::repo_root();
    let mut receipt = crate::json_boundary::read_json(
        &repo.join("validation_artifacts/ultragoal-audit/validator-receipt.json"),
    )
    .expect("current source-audit receipt exists for final packet proof fixture");
    receipt["status"] = json!("pass");
    receipt["commit"] = json!(current);
    receipt["target_revision"] = json!({"kind":"package_digest","value":current});
    receipt["claim_ceiling"] = json!("source_audit_pass_source_local_only");
    receipt["supported_claim_classes"] = json!(["source_local_audit_checks", "red_fixture_report"]);
    receipt["blocked_claim_classes"] = json!([
        "completion",
        "package_readiness",
        "review_readiness",
        "release_readiness",
        "final_packet_correctness",
        "update_goal_eligibility",
        "app_registry_or_reviewer_exposure"
    ]);
    receipt["validator_execution"]["command"]["exit"] = json!(0);
    mark_checks_pass(&mut receipt);
    keep_validator_generated_artifacts(&mut receipt);
    mark_red_fixtures_pass(&mut receipt);
    receipt
}

fn mark_checks_pass(receipt: &mut Value) {
    let Some(checks) = receipt.get_mut("checks").and_then(Value::as_object_mut) else {
        return;
    };
    for check in checks.values_mut() {
        check["status"] = json!("pass");
        if check.get("details").and_then(Value::as_str).is_none() {
            check["details"] = json!("test same-candidate source audit pass");
        }
    }
}

fn keep_validator_generated_artifacts(receipt: &mut Value) {
    let Some(artifacts) = receipt
        .get_mut("generated_artifacts")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    artifacts.retain(|artifact| {
        artifact
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.starts_with("validation_artifacts/ultragoal-audit/"))
    });
}

fn mark_red_fixtures_pass(receipt: &mut Value) {
    let Some(rows) = receipt
        .get_mut("red_fixtures")
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    for row in rows.values_mut() {
        row["status"] = json!("pass");
        if let Some(expected_error) = row.get("expected_error").cloned() {
            row["observed_error"] = expected_error;
        }
        if let Some(expected_check) = row.get("expected_failing_check").cloned() {
            row["observed_failing_check"] = expected_check;
        }
    }
}

#[test]
fn source_audit_helper_covers_missing_sections_and_detail_fill() {
    let mut no_checks = json!({});
    mark_checks_pass(&mut no_checks);

    let mut no_artifacts = json!({});
    keep_validator_generated_artifacts(&mut no_artifacts);

    let mut no_red_fixtures = json!({});
    mark_red_fixtures_pass(&mut no_red_fixtures);

    let mut receipt = json!({
        "checks": {
            "needs_detail": {"status":"fail"},
            "keeps_detail": {"status":"fail","details":"already present"}
        },
        "generated_artifacts": [
            {"path":"validation_artifacts/ultragoal-audit/validator-receipt.json"},
            {"path":"validation_artifacts/other.json"},
            {"path":null}
        ],
        "red_fixtures": {
            "red": {
                "status": "fail",
                "expected_error": "expected",
                "expected_failing_check": "check",
                "observed_error": "other",
                "observed_failing_check": "other-check"
            }
        }
    });
    mark_checks_pass(&mut receipt);
    mark_red_fixtures_pass(&mut receipt);
    assert_eq!(receipt["checks"]["needs_detail"]["status"], "pass");
    assert_eq!(
        receipt["checks"]["needs_detail"]["details"],
        "test same-candidate source audit pass"
    );
    assert_eq!(
        receipt["checks"]["keeps_detail"]["details"],
        "already present"
    );

    keep_validator_generated_artifacts(&mut receipt);
    assert_eq!(
        receipt["generated_artifacts"]
            .as_array()
            .expect("artifacts")
            .len(),
        1
    );
    assert_eq!(receipt["red_fixtures"]["red"]["status"], "pass");
    assert_eq!(receipt["red_fixtures"]["red"]["observed_error"], "expected");
    assert_eq!(
        receipt["red_fixtures"]["red"]["observed_failing_check"],
        "check"
    );
}
