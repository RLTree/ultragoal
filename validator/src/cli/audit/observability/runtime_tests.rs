use super::{RuntimeFacts, runtime};
use serde_json::json;

#[test]
fn audit_runtime_telemetry_reports_scheduler_resource_edges() {
    let mixed = runtime::telemetry(
        &json!({
            "scheduler_execution": [
                {
                    "worker_count": 0,
                    "task_count": 1,
                    "queue_depth": 0,
                    "cpu_ms": 5,
                    "memory_bytes": 50,
                    "io_bytes": 500,
                    "cache_mode": "a",
                    "resource_measurement_status": "ra"
                },
                {
                    "worker_count": 2,
                    "task_count": 1,
                    "queue_depth": 1,
                    "cpu_ms": 7,
                    "memory_bytes": 70,
                    "io_bytes": 700,
                    "cache_mode": "b",
                    "resource_measurement_status": "rb"
                }
            ]
        }),
        RuntimeFacts::from_elapsed_ms(9),
    );
    assert_eq!(mixed.cpu_ms, Some(12));
    assert_eq!(mixed.memory_bytes, Some(120));
    assert_eq!(mixed.io_bytes, Some(1200));
    assert_eq!(mixed.cache_mode, "mixed_cache_mode");
    assert_eq!(
        mixed.resource_measurement_status,
        "mixed_resource_measurement_status"
    );
    assert_eq!(mixed.saturation_status, "within_worker_capacity");

    let missing_worker = runtime::telemetry(
        &json!({"scheduler_execution":[{"task_count":1,"queue_depth":0}]}),
        RuntimeFacts::from_elapsed_ms(10),
    );
    assert_eq!(
        missing_worker.saturation_status,
        "scheduler_worker_count_missing"
    );
}
