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
