use crate::review::round::ReviewFailure;
use serde_json::Value;

const UNAVAILABLE: &str = "unavailable";
const UNAVAILABLE_CEILING: &str = "unavailable_no_runtime_configuration_claim";
const RUNTIME_CLAIM: &str = "reviewer_runtime_configuration";
const DISCOVERY_CLAIM: &str = "custom_agent_runtime_discovery";

pub(crate) fn receipt_errors(receipt: &Value, out: &mut Vec<ReviewFailure>) {
    if text(receipt, "review_authority") != "falsification_evidence_only" {
        push(out, "review_round_review_authority_invalid", "receipt");
    }
    let rows = receipt
        .get("reviewers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut unavailable = rows.is_empty();
    for row in &rows {
        unavailable |= metadata_errors(row.get("runtime_metadata"), out);
    }
    ceiling_errors(receipt, unavailable, out);
}

pub(crate) fn metadata_matches(row: &Value, spawn: &Value) -> bool {
    let row = row.get("runtime_metadata");
    row == spawn.get("runtime_metadata")
        && row
            .and_then(Value::as_object)
            .is_some_and(|metadata| metadata.len() == 1 && metadata["exposure"] == UNAVAILABLE)
}

fn metadata_errors(metadata: Option<&Value>, out: &mut Vec<ReviewFailure>) -> bool {
    const DETAIL: &str = "reviewer_runtime_metadata";
    let Some(metadata) = metadata.and_then(Value::as_object) else {
        push(out, "review_round_runtime_metadata_missing", DETAIL);
        return true;
    };
    match metadata.get("exposure").and_then(Value::as_str) {
        Some(UNAVAILABLE) => {
            if metadata.len() != 1 {
                push(
                    out,
                    "review_round_runtime_metadata_unexposed_values",
                    DETAIL,
                );
            }
            true
        }
        Some("host_exposed") => {
            push(
                out,
                "review_round_runtime_metadata_host_exposed_unbound",
                DETAIL,
            );
            true
        }
        _ => {
            push(out, "review_round_runtime_metadata_missing", DETAIL);
            true
        }
    }
}

fn ceiling_errors(receipt: &Value, unavailable: bool, out: &mut Vec<ReviewFailure>) {
    if !unavailable || text(receipt, "runtime_metadata_ceiling") != UNAVAILABLE_CEILING {
        push(
            out,
            "review_round_runtime_metadata_ceiling_mismatch",
            "receipt",
        );
    }
    let supported = claim_ids(&receipt["claim_ceiling"], "supported");
    let unsupported = claim_ids(&receipt["claim_ceiling"], "unsupported");
    if [RUNTIME_CLAIM, DISCOVERY_CLAIM]
        .iter()
        .any(|claim| supported.contains(claim) || !unsupported.contains(claim))
    {
        push(out, "review_round_runtime_metadata_overclaim", "receipt");
    }
}

fn claim_ids<'a>(value: &'a Value, key: &str) -> std::collections::BTreeSet<&'a str> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| row.get("claim_id").and_then(Value::as_str))
        .collect()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn push(out: &mut Vec<ReviewFailure>, error: &str, detail: &str) {
    out.push(ReviewFailure::new(
        "validator-execution-provenance",
        error,
        detail,
    ));
}
