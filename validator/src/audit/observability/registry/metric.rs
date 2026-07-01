use serde_json::{Map, Value};

pub(super) fn rows_match(rows: &Value, operation: &str) -> bool {
    rows.as_array()
        .is_some_and(|items| items.iter().any(|row| row_match(row, operation)))
}

fn row_match(row: &Value, operation: &str) -> bool {
    if let Some(metric) = row.get("metric").and_then(Value::as_object) {
        return object_match(row, metric, operation);
    }
    row.get("body")
        .and_then(Value::as_str)
        .and_then(|body| serde_json::from_str::<Value>(body).ok())
        .is_some_and(|body| body_match(&body, operation))
}

fn body_match(body: &Value, operation: &str) -> bool {
    body.pointer("/data/result")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|row| {
                row.get("metric")
                    .and_then(Value::as_object)
                    .is_some_and(|metric| object_match(row, metric, operation))
            })
        })
}

fn object_match(row: &Value, metric: &Map<String, Value>, operation: &str) -> bool {
    metric.get("__name__").and_then(Value::as_str) == Some("ultragoal_command_total")
        && operation_matches(row, metric, operation)
        && high_cardinality_labels()
            .into_iter()
            .all(|label| !metric.contains_key(label))
}

fn operation_matches(row: &Value, metric: &Map<String, Value>, operation: &str) -> bool {
    row.get("operation").and_then(Value::as_str) == Some(operation)
        || metric.get("operation").and_then(Value::as_str) == Some(operation)
}

fn high_cardinality_labels() -> [&'static str; 9] {
    [
        "candidate_digest",
        "run_id",
        "correlation_id",
        "trace_id",
        "span_id",
        "why_failed",
        "where_failed",
        "next_repair",
        "claim_impact",
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn matches_prometheus_body_rows_without_high_cardinality_labels() {
        let rows = json!([{
            "body": json!({
                "data": {
                    "result": [{
                        "metric": {
                            "__name__": "ultragoal_command_total",
                            "operation": "coverage.prove"
                        }
                    }]
                }
            }).to_string()
        }]);

        assert!(super::rows_match(&rows, "coverage.prove"));
    }

    #[test]
    fn rejects_wrong_metric_name_and_high_cardinality_labels() {
        let rows = json!([
            {
                "operation": "coverage.prove",
                "metric": {
                    "__name__": "other_total",
                    "operation": "coverage.prove"
                }
            },
            {
                "operation": "coverage.prove",
                "metric": {
                    "__name__": "ultragoal_command_total",
                    "operation": "coverage.prove",
                    "run_id": "run-high-cardinality"
                }
            }
        ]);

        assert!(!super::rows_match(&rows, "coverage.prove"));
    }

    #[test]
    fn accepts_operation_from_metric_when_row_omits_it() {
        let rows = json!([{
            "metric": {
                "__name__": "ultragoal_command_total",
                "operation": "schema.validate"
            }
        }]);

        assert!(super::rows_match(&rows, "schema.validate"));
    }
}
