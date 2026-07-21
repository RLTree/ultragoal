mod fixture;
mod tamper_cases;

use super::{CurrentAmendmentBinding, ExpectedArtifactBinding, validate_current};
use serde_json::json;

#[test]
fn current_log_yields_one_exact_current_amendment() {
    let binding = fixture::binding();
    let validated = validate_current(fixture::CURRENT_LOG, binding).expect("current amendment");
    assert_eq!(validated.amendment_id(), fixture::CURRENT_ID);
    assert_eq!(validated.amendment_hash(), fixture::CURRENT_HASH);
}

#[test]
fn current_strengthening_binds_old_and_new_contracts_and_every_output() {
    let mut rows = fixture::rows();
    let new_contract = format!("sha256:{}", "1".repeat(64));
    let second_digest = format!("sha256:{}", "2".repeat(64));
    rows[2]["change_class"] = json!("strengthens");
    rows[2]["new_contract_hash"] = json!(new_contract);
    rows[2]["backlog_updates"] = json!([
        {
            "path": "examples/generated/PRODUCT_SUCCESS_CONTRACT.json",
            "digest": "sha256:fc6c87b5888d250608bb5bc0b53155534d5b5638284e29076a3edd5d0768da55"
        },
        {"path": "schemas/product-success-brief.schema.json", "digest": second_digest}
    ]);
    let bytes = fixture::reseal(&mut rows);
    let amendment_hash = rows[2]["amendment_hash"].as_str().expect("amendment hash");
    let backlog = [
        ExpectedArtifactBinding {
            path: "examples/generated/PRODUCT_SUCCESS_CONTRACT.json",
            digest: "sha256:fc6c87b5888d250608bb5bc0b53155534d5b5638284e29076a3edd5d0768da55",
        },
        ExpectedArtifactBinding {
            path: "schemas/product-success-brief.schema.json",
            digest: &format!("sha256:{}", "2".repeat(64)),
        },
    ];
    let validated = validate_current(
        &bytes,
        CurrentAmendmentBinding {
            amendment_id: "AMEND-003",
            amendment_hash,
            previous_contract_hash:
                "sha256:6bd05cd382a2e8d1af10f6942ee64016f484983f5a98f118c4a3954ae8df6fa9",
            new_contract_hash: &format!("sha256:{}", "1".repeat(64)),
            change_class: "strengthens",
            backlog_updates: &backlog,
        },
    )
    .expect("current strengthening");
    assert_eq!(validated.amendment_hash(), amendment_hash);
}
