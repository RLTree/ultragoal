mod json;

#[cfg(unix)]
mod session;
#[cfg(unix)]
mod snapshot;
#[cfg(unix)]
mod sys;

#[cfg(all(test, unix))]
pub(crate) mod test_hooks;

#[cfg(unix)]
pub(crate) use session::Session;

#[cfg(not(unix))]
pub(crate) struct Session;

pub(crate) use json::parse_unique_json;

pub(crate) const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) const MAX_RESOURCE_BYTES: u64 = 64 * 1024 * 1024;

#[cfg(not(unix))]
impl Session {
    pub(crate) fn open(_root: &std::path::Path) -> Result<Self, String> {
        Err("anchored package reads are unavailable on this platform".to_string())
    }

    pub(crate) fn read(&mut self, _relative: &str, _maximum: u64) -> Result<Vec<u8>, String> {
        Err("anchored package reads are unavailable on this platform".to_string())
    }

    pub(crate) fn finish(&self) -> Result<(), String> {
        Err("anchored package reads are unavailable on this platform".to_string())
    }
}
