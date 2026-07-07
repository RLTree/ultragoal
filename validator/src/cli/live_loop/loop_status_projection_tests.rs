use super::receipt as subject;
use serde_json::json;

#[test]
fn loop_receipt_status_surfaces_do_not_collapse_validation_observability_or_speed() {
    let none = json!({"id":"none"});
    let product = json!({"id":"build_check"});
    let observability = json!({"id":"fmt_check"});
    let speed = json!({"id":"package_digest"});

    assert_eq!(subject::validation_status(&none), "pass");
    assert_eq!(subject::validation_status(&product), "fail");
    assert_eq!(
        subject::validation_cache_status("partial", &none),
        "reusable"
    );
    assert_eq!(
        subject::validation_cache_status("fail", &none),
        "not_reusable"
    );
    assert_eq!(
        subject::validation_cache_status("partial", &product),
        "not_reusable"
    );
    assert_eq!(subject::observability_status(&none), "pass");
    assert_eq!(subject::observability_status(&observability), "partial");
    assert_eq!(subject::speed_claim_status("pass", &none), "supported");
    assert_eq!(subject::speed_claim_status("partial", &none), "withheld");
    assert_eq!(subject::speed_claim_status("partial", &speed), "failed");
}
