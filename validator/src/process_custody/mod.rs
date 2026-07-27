//! Crate-private retained-process custody shared by host effects and routine work.

#[cfg(target_os = "macos")]
#[path = "darwin.rs"]
mod darwin;
#[cfg(target_os = "macos")]
#[path = "darwin_capture.rs"]
mod darwin_capture;
#[cfg(target_os = "macos")]
#[path = "darwin_cleanup.rs"]
pub(crate) mod darwin_cleanup;
#[cfg(target_os = "macos")]
#[path = "darwin_loaded.rs"]
mod darwin_loaded;
#[cfg(target_os = "macos")]
#[path = "darwin_process.rs"]
pub(crate) mod darwin_process;

#[cfg(target_os = "macos")]
pub(crate) use darwin_cleanup::cleanup_process;

#[cfg(target_os = "macos")]
pub(crate) use darwin::DarwinSuspendedProcess;
#[cfg(target_os = "macos")]
pub(crate) use darwin_process::{
    DarwinProcessFailure, DarwinProcessHooks, DarwinProcessPolicy, DarwinProcessTermination,
    execute_process,
};
