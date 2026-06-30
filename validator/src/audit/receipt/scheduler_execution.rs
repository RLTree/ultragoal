use serde_json::Value;

pub(super) fn evidence(metrics: &[crate::scheduler::Metrics], target_digest: &str) -> Value {
    Value::Array(
        metrics
            .iter()
            .map(|metric| {
                metric.to_value(
                    target_digest,
                    "supports_source_local_scheduler_timing_only_not_readiness",
                )
            })
            .collect(),
    )
}
