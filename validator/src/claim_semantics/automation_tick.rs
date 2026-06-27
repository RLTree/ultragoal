use crate::audit::contract::Failure;
use crate::claim_semantics::str_field;
use serde_json::Value;

pub(crate) fn check(tick: &Value, out: &mut Vec<Failure>) {
    let Some(clock) = tick_time(tick, "/freshness_policy/clock_at_validation", out) else {
        return;
    };
    let Some(last_tick) = tick_time(tick, "/last_tick_at", out) else {
        return;
    };
    let Some(last_success) = tick_time(tick, "/last_success_at", out) else {
        return;
    };
    if last_tick > clock || last_success > clock {
        out.push(Failure::new(
            "automation-tick-freshness",
            "automation_tick_future_timestamp",
            "tick timestamp",
        ));
        return;
    }
    let tick_budget = minutes(tick, "/freshness_policy/max_tick_age_minutes") * 60;
    let success_budget = minutes(tick, "/freshness_policy/max_success_age_minutes") * 60;
    let computed = if clock - last_tick <= tick_budget && clock - last_success <= success_budget {
        "fresh"
    } else {
        "stale"
    };
    if str_field(tick, "validator_computed_drift_verdict") != computed
        || str_field(tick, "drift_verdict") != computed
    {
        out.push(Failure::new(
            "automation-tick-freshness",
            "automation_timestamps_stale_but_claim_fresh",
            "computed verdict mismatch",
        ));
        return;
    }
    if computed == "stale" {
        out.push(Failure::new(
            "automation-tick-freshness",
            "automation_tick_stale_or_missing",
            "freshness budget",
        ));
    }
}

fn tick_time(tick: &Value, pointer: &str, out: &mut Vec<Failure>) -> Option<i64> {
    let raw = tick.pointer(pointer).and_then(Value::as_str).unwrap_or("");
    match crate::audit::clock::parse_iso_seconds(raw) {
        Some(value) => Some(value),
        None => {
            out.push(Failure::new(
                "automation-tick-freshness",
                "validation_clock_timestamp_malformed",
                pointer,
            ));
            None
        }
    }
}

fn minutes(tick: &Value, pointer: &str) -> i64 {
    tick.pointer(pointer).and_then(Value::as_i64).unwrap_or(0)
}
