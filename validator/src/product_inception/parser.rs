use super::model::{BriefV1, BriefV2};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) enum ParsedBrief {
    Historical(Box<BriefV1>),
    EvidenceLed(Box<BriefV2>),
}

pub(crate) fn parse(bytes: &[u8]) -> Result<ParsedBrief, &'static str> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| "brief_json_invalid")?;
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .ok_or("brief_schema_missing")?;
    match schema {
        "harness-ultragoal.product-success-brief.v1" => serde_json::from_value(value)
            .map(|brief| ParsedBrief::Historical(Box::new(brief)))
            .map_err(|_| "brief_v1_shape_invalid"),
        "harness-ultragoal.product-success-brief.v2" => serde_json::from_value(value)
            .map(|brief| ParsedBrief::EvidenceLed(Box::new(brief)))
            .map_err(|_| "brief_v2_shape_invalid"),
        _ => Err("brief_schema_unsupported"),
    }
}
