use super::EventStore;
use super::filesystem;
use super::format;
use super::limits::MAX_ROW_BYTES;
use super::locking::{LockDeadline, lock_exclusive};

impl EventStore {
    /// Explicitly remove every retained event while preserving the secured file identity.
    pub fn clear(&self) -> Result<bool, String> {
        let deadline = LockDeadline::for_store_operation()?;
        let initial_expected = self.identity.expected(&deadline)?;
        let Some(file) = filesystem::open_write_existing(self.identity.parent(), initial_expected)?
        else {
            return Ok(false);
        };
        lock_exclusive(&file, &deadline)?;
        let expected = self.resolve_expected_identity(initial_expected, &deadline)?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        file.set_len(0)
            .map_err(|_| "observe-clear-failed".to_owned())?;
        file.sync_data()
            .map_err(|_| "observe-clear-durability-failed".to_owned())?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        Ok(true)
    }

    /// Explicit recovery removes only an unterminated final row. Complete corrupt rows fail closed.
    pub fn recover_truncated_tail(&self) -> Result<usize, String> {
        let deadline = LockDeadline::for_store_operation()?;
        let initial_expected = self.identity.expected(&deadline)?;
        let Some(mut file) =
            filesystem::open_write_existing(self.identity.parent(), initial_expected)?
        else {
            return Ok(0);
        };
        lock_exclusive(&file, &deadline)?;
        let expected = self.resolve_expected_identity(initial_expected, &deadline)?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        let recovery_bound = self
            .max_store_bytes
            .checked_add(MAX_ROW_BYTES as u64)
            .ok_or_else(|| "observe-store-limit: recovery size overflow".to_owned())?;
        let bytes = filesystem::read_bounded(&mut file, recovery_bound)?;
        if bytes.is_empty() || bytes.ends_with(b"\n") {
            self.validate_decoded(&format::decode(&bytes, self.max_scan_rows)?)?;
            return Ok(0);
        }
        let retained = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        if retained as u64 > self.max_store_bytes {
            return Err("observe-store-limit: retained rows exceed byte bound".to_owned());
        }
        self.validate_decoded(&format::decode(&bytes[..retained], self.max_scan_rows)?)?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        file.set_len(retained as u64)
            .map_err(|_| "observe-recovery-failed".to_owned())?;
        file.sync_data()
            .map_err(|_| "observe-recovery-durability-failed".to_owned())?;
        self.identity.validate_bound(&self.path, &file, expected)?;
        Ok(bytes.len() - retained)
    }

    fn resolve_expected_identity(
        &self,
        initial_expected: Option<filesystem::FileIdentity>,
        deadline: &LockDeadline,
    ) -> Result<filesystem::FileIdentity, String> {
        match initial_expected {
            Some(expected) => Ok(expected),
            None => self.identity.expected(deadline)?.ok_or_else(|| {
                "observe-store-path-denied: store materialized outside append".to_owned()
            }),
        }
    }
}
