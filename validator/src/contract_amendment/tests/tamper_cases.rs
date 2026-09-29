use super::super::validate_current;
use super::fixture;
use serde_json::json;

#[test]
fn earlier_row_reseal_cannot_replace_adopted_history() {
    let mut rows = fixture::rows();
    rows[0]["created_at"] = json!("2026-07-15T23:30:49Z");
    let bytes = fixture::reseal(&mut rows);
    assert!(validate_current(&bytes, fixture::binding()).is_err());
}

#[test]
fn required_claim_removal_and_ceiling_reduction_fail_closed() {
    for mutation in 0..2 {
        let mut rows = fixture::rows();
        if mutation == 0 {
            rows[4]["removed_or_weakened_claim_ids"] = json!(["CL-SOURCE"]);
            rows[4]["derived_removed_or_weakened_claim_ids"] = json!(["CL-SOURCE"]);
        } else {
            rows[4]["after_claim_ceiling"]
                .as_array_mut()
                .expect("claim ceiling")
                .remove(0);
        }
        let bytes = fixture::reseal(&mut rows);
        assert!(validate_current(&bytes, fixture::binding()).is_err());
    }
}

#[test]
fn semantic_policy_tampering_fails_closed() {
    let mutations = [
        ("change_class", json!("weakens")),
        ("monotonicity", json!("weakens_with_user_approval")),
    ];
    for (field, value) in mutations {
        let mut rows = fixture::rows();
        rows[4][field] = value;
        let bytes = fixture::reseal(&mut rows);
        assert!(validate_current(&bytes, fixture::binding()).is_err());
    }
    let mut rows = fixture::rows();
    rows[4]["approval"] = json!({"required": true, "status": "missing"});
    let bytes = fixture::reseal(&mut rows);
    assert!(validate_current(&bytes, fixture::binding()).is_err());
}

#[test]
fn duplicate_current_row_and_backlog_substitution_fail_closed() {
    let mut duplicate = fixture::rows();
    duplicate.push(duplicate[4].clone());
    let bytes = fixture::reseal(&mut duplicate);
    assert!(validate_current(&bytes, fixture::binding()).is_err());

    let mut mismatch = fixture::rows();
    mismatch[4]["backlog_updates"][0]["path"] = json!("examples/generated/other.json");
    let bytes = fixture::reseal(&mut mismatch);
    assert!(validate_current(&bytes, fixture::binding()).is_err());
}

#[test]
fn omitted_reordered_and_extra_backlog_bindings_fail_closed() {
    for mutation in 0..3 {
        let mut rows = fixture::rows();
        let original = rows[4]["backlog_updates"][0].clone();
        match mutation {
            0 => rows[4]["backlog_updates"] = json!([]),
            1 => {
                rows[4]["backlog_updates"] = json!([
                    {"path": "schemas/other.json", "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"},
                    original
                ])
            }
            _ => {
                rows[4]["backlog_updates"] = json!([
                    original,
                    {"path": "schemas/other.json", "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"}
                ])
            }
        }
        let bytes = fixture::reseal(&mut rows);
        assert!(validate_current(&bytes, fixture::binding()).is_err());
    }
}
