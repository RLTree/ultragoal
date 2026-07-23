use super::super::{CurrentAmendmentBinding, ExpectedArtifactBinding};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub(super) const CURRENT_LOG: &[u8] = include_bytes!("../../../../AMENDMENTS.jsonl");
pub(super) const CURRENT_ID: &str = "AMEND-005";
pub(super) const CURRENT_HASH: &str =
    "sha256:39f51d83b90508087e45459f81add8094c2c455fb669e9bbd5885b3909d5338d";
const PREVIOUS_CONTRACT_HASH: &str =
    "sha256:20dadce2e50ef92fa4f19614f8fc70ae472ad32064561fca2208ca213cf0685b";
const CONTRACT_HASH: &str =
    "sha256:6488e75196f28582adac4ea6079ce0878a6ad7059b720eaea68b08b780874b7a";
const BACKLOG: [ExpectedArtifactBinding<'static>; 3] = [
    ExpectedArtifactBinding {
        path: "examples/generated/PRODUCT_SUCCESS_CONTRACT.json",
        digest: "sha256:0cbaa1ba5ff32448f9be306ee8bcf5a26c878ee4ec77a1b3b238eb231e0334a3",
    },
    ExpectedArtifactBinding {
        path: "PRODUCT_SUCCESS_BRIEF.json",
        digest: "sha256:064037bbd4f2a2234982a458ec5afb3ef35d8383b2f52680c1fbed1dc4adcd78",
    },
    ExpectedArtifactBinding {
        path: "docs/ultragoal-successor-live/root-decisions/AGENTIC-ENGINEERING-V3-LIFECYCLE-ADVISORY-004.json",
        digest: "sha256:70b9ecaee5b4a5e38f12442826ebf08fbf607a6ebd6bdbac5c5d602cd18b73b1",
    },
];

pub(super) fn binding() -> CurrentAmendmentBinding<'static> {
    CurrentAmendmentBinding {
        amendment_id: CURRENT_ID,
        amendment_hash: CURRENT_HASH,
        previous_contract_hash: PREVIOUS_CONTRACT_HASH,
        new_contract_hash: CONTRACT_HASH,
        change_class: "strengthens",
        backlog_updates: &BACKLOG,
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
