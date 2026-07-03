use crate::cli::observe;
use serde_json::json;

#[test]
fn observe_csv_contract_renders_supported_claim_arrays() {
    assert_eq!(
        observe::csv_for_test(&json!({"supported_claims":["logs_query","traces_query"]})),
        "logs_query,traces_query"
    );
}
