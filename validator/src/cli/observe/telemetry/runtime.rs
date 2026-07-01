#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeTelemetry {
    pub(crate) duration_ms: u64,
    pub(crate) worker_count: usize,
    pub(crate) task_count: usize,
    pub(crate) queue_depth: usize,
    pub(crate) cpu_ms: Option<u64>,
    pub(crate) memory_bytes: Option<u64>,
    pub(crate) io_bytes: Option<u64>,
    pub(crate) cache_mode: String,
    pub(crate) resource_measurement_status: String,
    pub(crate) retry_count: u64,
    pub(crate) backoff_ms: u64,
    pub(crate) saturation_status: String,
    pub(crate) repair_anchor_before: String,
    pub(crate) repair_anchor_after: String,
}

impl RuntimeTelemetry {
    pub(crate) fn uninstrumented() -> Self {
        Self {
            duration_ms: 0,
            worker_count: 0,
            task_count: 0,
            queue_depth: 0,
            cpu_ms: None,
            memory_bytes: None,
            io_bytes: None,
            cache_mode: "not_reported".to_string(),
            resource_measurement_status: "runtime_not_instrumented_for_command_yet".to_string(),
            retry_count: 0,
            backoff_ms: 0,
            saturation_status: "not_measured".to_string(),
            repair_anchor_before: "none".to_string(),
            repair_anchor_after: "none".to_string(),
        }
    }
}
