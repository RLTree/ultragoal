use super::scanner_syscall_adapter::{self, DirectoryStream};
use super::{
    injected_readdir_error, next_scan_ordinal, record_directory_entry, record_directory_scan,
    record_failed_stream_close, record_post_terminal_call, record_readdir_call,
    record_readdir_error,
};
use crate::plugin_product::agent_discovery::error::AgentDiscoveryError;
use crate::plugin_product::agent_discovery::filesystem::unsafe_entry;
use std::os::fd::RawFd;

pub(super) struct DirectoryScanner {
    stream: DirectoryStream,
    scan_ordinal: usize,
    yielded_entries: usize,
    terminal: bool,
    failed: bool,
    closed: bool,
}

impl DirectoryScanner {
    pub(super) fn open(directory_fd: RawFd) -> Result<Self, AgentDiscoveryError> {
        let stream = scanner_syscall_adapter::open(directory_fd).map_err(|_| unsafe_entry())?;
        record_directory_scan();
        Ok(Self {
            stream,
            scan_ordinal: next_scan_ordinal(),
            yielded_entries: 0,
            terminal: false,
            failed: false,
            closed: false,
        })
    }

    pub(super) fn next(&mut self) -> Result<Option<Vec<u8>>, AgentDiscoveryError> {
        if self.terminal || self.closed {
            record_post_terminal_call();
            return Err(unsafe_entry());
        }
        loop {
            record_readdir_call();
            let injected_error = injected_readdir_error(self.scan_ordinal, self.yielded_entries);
            let entry = match scanner_syscall_adapter::read(&mut self.stream, injected_error) {
                Ok(entry) => entry,
                Err(_) => {
                    self.terminal = true;
                    self.failed = true;
                    record_readdir_error();
                    return Err(unsafe_entry());
                }
            };
            let Some(bytes) = entry else {
                self.terminal = true;
                return Ok(None);
            };
            if bytes == b"." || bytes == b".." {
                continue;
            }
            self.yielded_entries += 1;
            record_directory_entry();
            return Ok(Some(bytes));
        }
    }

    pub(super) fn close(&mut self) -> Result<(), AgentDiscoveryError> {
        if self.closed {
            return Err(unsafe_entry());
        }
        if self.failed {
            record_failed_stream_close();
        }
        self.closed = true;
        scanner_syscall_adapter::close(&mut self.stream).map_err(|_| unsafe_entry())
    }
}

impl Drop for DirectoryScanner {
    fn drop(&mut self) {
        if !self.closed {
            if self.failed {
                record_failed_stream_close();
            }
            self.closed = true;
            let _ = scanner_syscall_adapter::close(&mut self.stream);
        }
    }
}
