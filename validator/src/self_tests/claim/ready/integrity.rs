use crate::audit::contract::Failure;
use serde_json::{Value, json};

fn errors(out: &[Failure]) -> Vec<&str> {
    out.iter().map(|failure| failure.error.as_str()).collect()
}

fn ready() -> Value {
    json!({
        "lane_id":"lane-1",
        "ready":true,
        "validator_run_id":"run",
        "provenance":{"validator_run_id":"run"}
    })
}

#[test]
fn ready_integrity_normalizes_dependency_receipt_digests() {
    let lane_with_dependency = json!({
        "id":"lane-1",
        "ready_receipt":{"digest":crate::self_tests::boundaries::support::sha('1')},
        "dependencies":[{
            "evidence_digest":crate::self_tests::boundaries::support::sha('2'),
            "upstream_ready_receipt":{"digest":crate::self_tests::boundaries::support::sha('3')}
        }]
    });
    let mut out = Vec::new();
    crate::claim_semantics::ready::join::check_ready_integrity(
        &json!({"lanes":[lane_with_dependency]}),
        &ready(),
        &mut out,
    );
    assert!(errors(&out).contains(&"ready_receipt_not_lane_bound"));
}
