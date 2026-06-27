use crate::claim_semantics::str_field;
use serde_json::Value;

pub(crate) fn actor_freshness_error(lane: &Value, run_at: i64) -> Option<String> {
    let binding = &lane["actor_binding"];
    if str_field(binding, "actor_status") != "current" {
        return Some("stale_actor_identity".to_string());
    }
    let validated =
        match crate::audit::clock::parse_iso_seconds(&str_field(binding, "validated_at")) {
            Some(value) => value,
            None => return Some("actor_validation_timestamp_malformed".to_string()),
        };
    let heartbeat = match crate::audit::clock::parse_iso_seconds(&str_field(lane, "last_heartbeat"))
    {
        Some(value) => value,
        None => return Some("lane_heartbeat_timestamp_malformed".to_string()),
    };
    let ttl = 14 * 24 * 60 * 60;
    if validated > run_at {
        return Some("future_actor_identity_validation".to_string());
    }
    if heartbeat > run_at {
        return Some("future_lane_heartbeat".to_string());
    }
    if run_at - heartbeat > ttl {
        return Some("stale_lane_heartbeat".to_string());
    }
    if validated < heartbeat {
        return Some("actor_identity_older_than_heartbeat".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::actor_freshness_error;
    use serde_json::{Value, json};

    fn lane(validated_at: &str, heartbeat: &str) -> Value {
        json!({
            "actor_binding": {
                "actor_status": "current",
                "validated_at": validated_at
            },
            "last_heartbeat": heartbeat
        })
    }

    fn run_at() -> i64 {
        crate::audit::clock::parse_iso_seconds("2026-06-25T00:00:00Z").expect("run clock")
    }

    #[test]
    fn actor_freshness_rejects_stale_future_and_malformed_identity() {
        assert_eq!(
            actor_freshness_error(
                &json!({"actor_binding":{"actor_status":"retired"}}),
                run_at()
            ),
            Some("stale_actor_identity".to_string())
        );
        assert_eq!(
            actor_freshness_error(&lane("not-a-time", "2026-06-25T00:00:00Z"), run_at()),
            Some("actor_validation_timestamp_malformed".to_string())
        );
        assert_eq!(
            actor_freshness_error(&lane("2026-06-25T00:00:00Z", "not-a-time"), run_at()),
            Some("lane_heartbeat_timestamp_malformed".to_string())
        );
        assert_eq!(
            actor_freshness_error(
                &lane("2026-06-25T00:00:01Z", "2026-06-25T00:00:00Z"),
                run_at()
            ),
            Some("future_actor_identity_validation".to_string())
        );
        assert_eq!(
            actor_freshness_error(
                &lane("2026-06-25T00:00:00Z", "2026-06-25T00:00:01Z"),
                run_at()
            ),
            Some("future_lane_heartbeat".to_string())
        );
        assert_eq!(
            actor_freshness_error(
                &lane("2026-06-25T00:00:00Z", "2026-06-01T00:00:00Z"),
                run_at()
            ),
            Some("stale_lane_heartbeat".to_string())
        );
        assert_eq!(
            actor_freshness_error(
                &lane("2026-06-24T00:00:00Z", "2026-06-24T00:00:01Z"),
                run_at()
            ),
            Some("actor_identity_older_than_heartbeat".to_string())
        );
        assert_eq!(
            actor_freshness_error(
                &lane("2026-06-24T00:00:00Z", "2026-06-23T00:00:00Z"),
                run_at()
            ),
            None
        );
    }
}
