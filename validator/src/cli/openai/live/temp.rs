use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn write_temp(path: &Path, bytes: &[u8], failures: &mut Vec<String>) {
    if std::fs::write(path, bytes).is_err() {
        failures.push("openai_live_temp_write_failed".to_string());
    }
}

pub(super) fn read_temp(path: &Path, failures: &mut Vec<String>) -> String {
    match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            failures.push("openai_live_temp_read_failed".to_string());
            String::new()
        }
    }
}

pub(super) struct TempPaths {
    pub(super) request: PathBuf,
    pub(super) response: PathBuf,
    pub(super) headers: PathBuf,
}

impl TempPaths {
    pub(super) fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!("ultragoal-openai-live-{stamp}"));
        Self {
            request: base.with_extension("request.json"),
            response: base.with_extension("response.json"),
            headers: base.with_extension("headers.txt"),
        }
    }

    pub(super) fn cleanup(&self) {
        let _ = std::fs::remove_file(&self.request);
        let _ = std::fs::remove_file(&self.response);
        let _ = std::fs::remove_file(&self.headers);
    }
}
