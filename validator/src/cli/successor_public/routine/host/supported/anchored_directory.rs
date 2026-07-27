#[path = "anchored_opening.rs"]
mod anchored_opening;
#[path = "exclusive_publish.rs"]
mod exclusive_publish;
#[path = "file_descriptor.rs"]
mod file_descriptor;
#[path = "identity_verification.rs"]
mod identity_verification;
#[path = "process_lock.rs"]
mod process_lock;
#[path = "regular_file.rs"]
mod regular_file;

pub(crate) use process_lock::write_lock_marker;
