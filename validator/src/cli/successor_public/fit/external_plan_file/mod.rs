use std::path::Path;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;

#[cfg(not(unix))]
mod platform {
    use std::path::Path;

    pub(super) fn read_immutable_plan(
        _path: &Path,
        _maximum: u64,
    ) -> Result<Vec<u8>, &'static str> {
        Err("plan-input-platform-unsupported")
    }
}

pub(super) fn read_immutable_plan(path: &Path, maximum: u64) -> Result<Vec<u8>, &'static str> {
    platform::read_immutable_plan(path, maximum)
}

#[cfg(test)]
mod tests;
