use super::error::ContextError;
use super::read_session::{MAX_READ_SESSION_BYTES, ReadSession};

fn exhausted() -> ContextError {
    ContextError::PathDenied(format!(
        "read session exceeds {MAX_READ_SESSION_BYTES} total bytes"
    ))
}

impl ReadSession {
    pub(crate) fn charge(&self, bytes: u64) -> Result<(), ContextError> {
        let total = self.bytes_read.get().saturating_add(bytes);
        if total > MAX_READ_SESSION_BYTES {
            return Err(exhausted());
        }
        self.bytes_read.set(total);
        Ok(())
    }

    pub(super) fn require_read_capacity(&self) -> Result<(), ContextError> {
        if self.bytes_read.get() >= MAX_READ_SESSION_BYTES {
            return Err(exhausted());
        }
        Ok(())
    }

    pub(super) fn reserve_read_capacity(&self, requested: usize) -> Result<usize, ContextError> {
        self.require_read_capacity()?;
        let remaining = MAX_READ_SESSION_BYTES - self.bytes_read.get();
        let reserved = remaining.min(requested as u64) as usize;
        self.bytes_read
            .set(self.bytes_read.get().saturating_add(reserved as u64));
        Ok(reserved)
    }

    pub(super) fn commit_read_capacity(&self, reserved: usize, actual: usize) {
        debug_assert!(actual <= reserved);
        self.bytes_read
            .set(self.bytes_read.get() - (reserved - actual) as u64);
    }
}
