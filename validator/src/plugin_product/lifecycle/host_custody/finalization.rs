/// A one-use proof that the transferred lifecycle reached a durable terminal
/// disposition. It is intentionally opaque outside lifecycle custody: callers
/// cannot release a staged executable before settlement, nor before an
/// ambiguous result has completed its authorized recovery.
pub(crate) struct HostLifecycleFinalization {
    #[cfg(not(test))]
    record: HostLifecycleRecord,
}

impl HostLifecycleFinalization {
    #[cfg(not(test))]
    pub(crate) fn matches(&self, binding: &HostEffectExecutionBinding) -> bool {
        &self.record == binding.record()
    }
}
