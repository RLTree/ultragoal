use std::cell::Cell;

use super::scanner_syscall_adapter::{INTERRUPTED_ERROR, IO_ERROR};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReaddirTestFault {
    Io,
    Interrupted,
}

impl ReaddirTestFault {
    fn error_number(self) -> i32 {
        match self {
            Self::Io => IO_ERROR,
            Self::Interrupted => INTERRUPTED_ERROR,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TestIoCounts {
    pub(crate) directory_entries: usize,
    pub(crate) directory_scans: usize,
    pub(crate) metadata_probes: usize,
    pub(crate) file_open_attempts: usize,
    pub(crate) readdir_calls: usize,
    pub(crate) readdir_errors: usize,
    pub(crate) failed_stream_closes: usize,
    pub(crate) post_terminal_calls: usize,
}

thread_local! {
    static TEST_IO_COUNTS: Cell<TestIoCounts> = const { Cell::new(TestIoCounts {
        directory_entries: 0,
        directory_scans: 0,
        metadata_probes: 0,
        file_open_attempts: 0,
        readdir_calls: 0,
        readdir_errors: 0,
        failed_stream_closes: 0,
        post_terminal_calls: 0,
    }) };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TestReaddirFault {
    scan_ordinal: usize,
    after_entries: usize,
    error_number: i32,
    probe_post_error: bool,
    triggered: bool,
}

thread_local! {
    static TEST_SCAN_ORDINAL: Cell<usize> = const { Cell::new(0) };
    static TEST_READDIR_FAULT: Cell<Option<TestReaddirFault>> = const { Cell::new(None) };
}

macro_rules! record_count {
    ($name:ident, $field:ident) => {
        pub(super) fn $name() {
            TEST_IO_COUNTS.with(|counts| {
                let mut current = counts.get();
                current.$field += 1;
                counts.set(current);
            });
        }
    };
}

record_count!(record_directory_entry, directory_entries);
record_count!(record_directory_scan, directory_scans);
record_count!(record_metadata_probe, metadata_probes);
record_count!(record_file_open_attempt, file_open_attempts);
record_count!(record_readdir_call, readdir_calls);
record_count!(record_readdir_error, readdir_errors);
record_count!(record_failed_stream_close, failed_stream_closes);
record_count!(record_post_terminal_call, post_terminal_calls);

pub(super) fn next_scan_ordinal() -> usize {
    TEST_SCAN_ORDINAL.with(|ordinal| {
        let current = ordinal.get();
        ordinal.set(current + 1);
        current
    })
}

pub(super) fn injected_readdir_error(scan_ordinal: usize, yielded_entries: usize) -> Option<i32> {
    TEST_READDIR_FAULT.with(|fault| {
        let mut current = fault.get()?;
        if current.triggered
            || current.scan_ordinal != scan_ordinal
            || current.after_entries != yielded_entries
        {
            return None;
        }
        current.triggered = true;
        fault.set(Some(current));
        Some(current.error_number)
    })
}

pub(super) fn probe_post_error_after_fault() -> bool {
    TEST_READDIR_FAULT.with(|fault| {
        fault
            .get()
            .is_some_and(|fault| fault.triggered && fault.probe_post_error)
    })
}

pub(crate) fn reset_test_io_counts() {
    TEST_IO_COUNTS.with(|counts| counts.set(TestIoCounts::default()));
    TEST_SCAN_ORDINAL.with(|ordinal| ordinal.set(0));
    TEST_READDIR_FAULT.with(|fault| fault.set(None));
}

pub(crate) fn test_io_counts() -> TestIoCounts {
    TEST_IO_COUNTS.with(Cell::get)
}

pub(crate) fn set_test_readdir_fault(
    scan_ordinal: usize,
    after_entries: usize,
    fault: ReaddirTestFault,
    probe_post_error: bool,
) {
    let error_number = fault.error_number();
    TEST_SCAN_ORDINAL.with(|ordinal| ordinal.set(0));
    TEST_READDIR_FAULT.with(|fault| {
        fault.set(Some(TestReaddirFault {
            scan_ordinal,
            after_entries,
            error_number,
            probe_post_error,
            triggered: false,
        }));
    });
}

pub(crate) fn test_readdir_fault_triggered() -> bool {
    TEST_READDIR_FAULT.with(|fault| fault.get().is_some_and(|fault| fault.triggered))
}
