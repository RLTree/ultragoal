use super::{RuntimeFacts, red};
use serde_json::json;
use std::fs;

const RECEIPT: &str = "validation_artifacts/observability/red-fixture-report.json";

#[test]
fn red_fixture_report_observability_receipt_supports_only_red_report() {
    let root = crate::self_tests::boundaries::support::temp_root("red-report-observe");
    fs::create_dir_all(root.join("validation_artifacts/ultragoal-audit")).expect("audit dir");
    crate::json_boundary::write_json(
        &root.join("plugin-manifest-draft.json"),
        &json!({"resources":[]}),
    )
    .expect("manifest");
    let candidate = crate::package::inventory::package_digest(&root).expect("candidate");
    let report = root.join("validation_artifacts/ultragoal-audit/red-fixture-report.json");
    crate::json_boundary::write_json(
        &report,
        &json!({
            "status":"pass",
            "target_revision":{"kind":"package_digest","value":candidate},
            "red_fixtures":{"red-one":{"status":"pass"}}
        }),
    )
    .expect("red report");
    red::write_report(&root, &report, None, RuntimeFacts::from_elapsed_ms(5))
        .expect("observability");
    let value = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("red receipt");
    assert_eq!(value["status"], "pass");
    assert_eq!(value["operation"], "red_fixture.report");
    assert_eq!(value["claim_id"], "red_fixture_report");
    assert!(
        value["supported_claims"]
            .as_array()
            .expect("supported")
            .iter()
            .any(|item| item.as_str() == Some("red_fixture_report"))
    );
    assert!(
        value["blocked_claims"]
            .as_array()
            .expect("blocked")
            .iter()
            .any(|item| item.as_str() == Some("update_goal_eligibility"))
    );

    crate::json_boundary::write_json(
        &report,
        &json!({"status":"fail","red_fixtures":{"red-one":{"status":"pass"}}}),
    )
    .expect("empty failure red report");
    red::write_report(&root, &report, None, RuntimeFacts::from_elapsed_ms(6))
        .expect("empty failure observe");
    let empty = crate::json_boundary::read_json(&root.join(RECEIPT)).expect("empty receipt");
    assert_eq!(
        empty["why_failed"],
        "red fixture report missing, malformed, stale, or not pass"
    );
    fs::remove_dir_all(root).expect("cleanup");
}
