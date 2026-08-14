use serde_json::{Value, json};

const SCHEMA: &str = "RoutineBehaviorRefusal-v1";
const BEHAVIOR: &str = "rust-source-syntax-v1";

pub(crate) fn refusal_json(reason: &'static str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": SCHEMA,
        "behavior_id": BEHAVIOR,
        "status": "refused",
        "reason": reason,
    }))
    .expect("closed routine refusal is serializable")
}

pub(crate) fn trusted_refusal_failure_code(bytes: &[u8]) -> Option<&'static str> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object()?;
    if object.len() != 4
        || object.get("schema_version")?.as_str()? != SCHEMA
        || object.get("behavior_id")?.as_str()? != BEHAVIOR
        || object.get("status")?.as_str()? != "refused"
    {
        return None;
    }
    Some(match object.get("reason")?.as_str()? {
        "legacy_selector_present" => "MEDIATOR-BEHAVIOR-LEGACY-SELECTOR",
        "invocation_not_authorized" => "MEDIATOR-BEHAVIOR-INVOCATION-REFUSED",
        "sandbox_input_protocol_invalid" => "MEDIATOR-SANDBOX-INPUT-INVALID",
        "sandbox_profile_invalid" => "MEDIATOR-SANDBOX-PROFILE-INVALID",
        "sandbox_activation_unavailable" => "MEDIATOR-SANDBOX-ACTIVATION-UNAVAILABLE",
        "behavior_frame_invalid" => "MEDIATOR-BEHAVIOR-FRAME-INVALID",
        "rust_source_syntax_refused" => "MEDIATOR-RUST-SOURCE-SYNTAX-REFUSED",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_closed_refusal_shape_maps_to_a_public_failure_code() {
        for (reason, expected) in [
            (
                "legacy_selector_present",
                "MEDIATOR-BEHAVIOR-LEGACY-SELECTOR",
            ),
            (
                "invocation_not_authorized",
                "MEDIATOR-BEHAVIOR-INVOCATION-REFUSED",
            ),
            (
                "sandbox_input_protocol_invalid",
                "MEDIATOR-SANDBOX-INPUT-INVALID",
            ),
            (
                "sandbox_profile_invalid",
                "MEDIATOR-SANDBOX-PROFILE-INVALID",
            ),
            (
                "sandbox_activation_unavailable",
                "MEDIATOR-SANDBOX-ACTIVATION-UNAVAILABLE",
            ),
            ("behavior_frame_invalid", "MEDIATOR-BEHAVIOR-FRAME-INVALID"),
            (
                "rust_source_syntax_refused",
                "MEDIATOR-RUST-SOURCE-SYNTAX-REFUSED",
            ),
        ] {
            assert_eq!(
                trusted_refusal_failure_code(&refusal_json(reason)),
                Some(expected)
            );
        }

        let bytes = refusal_json("sandbox_activation_unavailable");
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["extra"] = Value::Bool(true);
        assert_eq!(
            trusted_refusal_failure_code(&serde_json::to_vec(&value).unwrap()),
            None
        );
        assert_eq!(
            trusted_refusal_failure_code(&refusal_json("future_reason")),
            None
        );
    }
}
