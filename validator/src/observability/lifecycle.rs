use super::EventStore;
use super::filesystem;
use super::format;
use super::limits::MAX_ROW_BYTES;

impl EventStore {
    /// Explicitly remove every retained event while preserving the secured file identity.
    pub fn clear(&self) -> Result<bool, String> {
        let Some(file) =
            filesystem::open_write_existing(self.identity.parent(), self.identity.expected()?)?
        else {
            return Ok(false);
        };
        lock_exclusive(&file)?;
        self.identity.validate_bound(&self.path, &file)?;
        file.set_len(0)
            .map_err(|_| "observe-clear-failed".to_owned())?;
        file.sync_data()
            .map_err(|_| "observe-clear-durability-failed".to_owned())?;
        self.identity.validate_bound(&self.path, &file)?;
        Ok(true)
    }

    /// Explicit recovery removes only an unterminated final row. Complete corrupt rows fail closed.
    pub fn recover_truncated_tail(&self) -> Result<usize, String> {
        let Some(mut file) =
            filesystem::open_write_existing(self.identity.parent(), self.identity.expected()?)?
        else {
            return Ok(0);
        };
        lock_exclusive(&file)?;
        self.identity.validate_bound(&self.path, &file)?;
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
        self.identity.validate_bound(&self.path, &file)?;
        file.set_len(retained as u64)
            .map_err(|_| "observe-recovery-failed".to_owned())?;
        file.sync_data()
            .map_err(|_| "observe-recovery-durability-failed".to_owned())?;
        self.identity.validate_bound(&self.path, &file)?;
        Ok(bytes.len() - retained)
    }
}

fn lock_exclusive(file: &std::fs::File) -> Result<(), String> {
    file.lock()
        .map_err(|_| "observe-store-lock-failed".to_owned())
}
