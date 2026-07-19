use super::super::CurrentAmendmentBinding;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) const CURRENT_LOG: &[u8] = include_bytes!("../../../../AMENDMENTS.jsonl");
pub(super) const CURRENT_ID: &str = "AMEND-003";
pub(super) const CURRENT_HASH: &str =
    "sha256:ea134939717ab2422a444f40eed9ca6b388e2d86846a74ce2f0aff823cb95600";
const CONTRACT_HASH: &str =
    "sha256:6bd05cd382a2e8d1af10f6942ee64016f484983f5a98f118c4a3954ae8df6fa9";
const OUTPUT: &str = "examples/generated/PRODUCT_SUCCESS_CONTRACT.json";
const OUTPUT_HASH: &str = "sha256:fc6c87b5888d250608bb5bc0b53155534d5b5638284e29076a3edd5d0768da55";

pub(super) fn binding() -> CurrentAmendmentBinding<'static> {
    CurrentAmendmentBinding {
        amendment_id: CURRENT_ID,
        amendment_hash: CURRENT_HASH,
        contract_hash: CONTRACT_HASH,
        output_path: OUTPUT,
        output_hash: OUTPUT_HASH,
    }
}

pub(super) fn rows() -> Vec<Value> {
    std::str::from_utf8(CURRENT_LOG)
        .expect("UTF-8 log")
        .lines()
        .map(|line| serde_json::from_str(line).expect("amendment row"))
        .collect()
}

pub(super) fn reseal(rows: &mut [Value]) -> Vec<u8> {
    for index in 0..rows.len() {
        if index > 0 {
            let previous_hash = rows[index - 1]["amendment_hash"].clone();
            let previous_contract = rows[index - 1]["new_contract_hash"].clone();
            rows[index]["previous_amendment_hash"] = previous_hash;
            rows[index]["previous_contract_hash"] = previous_contract;
        }
        rows[index]["amendment_hash"] = Value::String(canonical_hash(&rows[index]));
    }
    let mut bytes = rows
        .iter()
        .map(|row| serde_json::to_string(row).expect("serialize amendment"))
        .collect::<Vec<_>>()
        .join("\n")
        .into_bytes();
    bytes.push(b'\n');
    bytes
}

fn canonical_hash(row: &Value) -> String {
    let mut value = row.clone();
    value
        .as_object_mut()
        .expect("amendment object")
        .remove("amendment_hash");
    let bytes = serde_json::to_vec(&value).expect("canonical amendment");
    format!("sha256:{:x}", Sha256::digest(bytes))
}
