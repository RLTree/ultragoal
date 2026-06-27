use serde_json::Value;

use crate::claim_semantics::str_field;

pub(super) fn blocks_isolation(lane: &Value) -> bool {
    !is_terminal(&str_field(lane, "status"))
}

pub(super) fn is_terminal(status: &str) -> bool {
    matches!(status, "merged" | "superseded" | "abandoned")
}
