mod directory;
mod directory_syscall_adapter;
mod identity;
mod identity_syscall_adapter;
mod root;
mod scanner;
mod scanner_syscall_adapter;
#[cfg(test)]
mod test_instrumentation;

use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(test)]
pub(crate) use test_instrumentation::{
    ReaddirTestFault, reset_test_io_counts, set_test_readdir_fault, test_io_counts,
    test_readdir_fault_triggered,
};
#[cfg(test)]
use test_instrumentation::{
    injected_readdir_error, next_scan_ordinal, probe_post_error_after_fault,
    record_directory_entry, record_directory_scan, record_failed_stream_close,
    record_file_open_attempt, record_metadata_probe, record_post_terminal_call,
    record_readdir_call, record_readdir_error,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FileFingerprint {
    len: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    device: u64,
    inode: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DirectoryFingerprint {
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    device: u64,
    inode: u64,
}

#[derive(Debug)]
struct DirectoryHandle {
    file: File,
}

#[derive(Clone, Debug)]
pub(crate) struct AnchoredDirectory {
    handle: Arc<DirectoryHandle>,
    fingerprint: DirectoryFingerprint,
}

#[derive(Clone, Debug)]
pub(crate) struct AnchoredRoot {
    root_path: PathBuf,
    directory: AnchoredDirectory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SecureFile {
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256: String,
    pub(crate) fingerprint: FileFingerprint,
}

#[cfg(not(test))]
fn record_directory_entry() {}
#[cfg(not(test))]
fn record_directory_scan() {}
#[cfg(not(test))]
fn record_metadata_probe() {}
#[cfg(not(test))]
fn record_file_open_attempt() {}
#[cfg(not(test))]
fn record_readdir_call() {}
#[cfg(not(test))]
fn record_readdir_error() {}
#[cfg(not(test))]
fn record_failed_stream_close() {}
#[cfg(not(test))]
fn record_post_terminal_call() {}
#[cfg(not(test))]
fn next_scan_ordinal() -> usize {
    0
}
#[cfg(not(test))]
fn injected_readdir_error(_scan_ordinal: usize, _yielded_entries: usize) -> Option<i32> {
    None
}
