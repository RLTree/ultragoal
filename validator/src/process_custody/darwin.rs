//! Neutral Darwin suspended launch kernel.

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::os::fd::RawFd;
use std::path::Path;

pub(crate) struct DarwinSuspendedProcess {
    pub(crate) pid: libc::pid_t,
    pub(crate) group: libc::pid_t,
    stdin: Option<File>,
    stdout: Option<File>,
    stderr: Option<File>,
    settled: bool,
}

impl DarwinSuspendedProcess {
    pub(crate) fn from_spawn(pid: libc::pid_t, stdin: File, stdout: File, stderr: File) -> Self {
        Self {
            pid,
            group: pid,
            stdin: Some(stdin),
            stdout: Some(stdout),
            stderr: Some(stderr),
            settled: false,
        }
    }

    pub(crate) fn group(&self) -> libc::pid_t {
        self.group
    }

    pub(crate) fn validate_loaded_vnode(
        &self,
        expected_path: &Path,
        expected_device: u64,
        expected_inode: u64,
    ) -> io::Result<()> {
        super::darwin_loaded::validate_loaded_vnode(
            self.pid,
            expected_path,
            expected_device,
            expected_inode,
        )
    }

    pub(crate) fn mark_settled(&mut self) {
        self.settled = true;
    }

    pub(crate) fn take_pipes(&mut self) -> io::Result<(File, File, File)> {
        Ok((
            self.stdin
                .take()
                .ok_or_else(|| io::Error::other("stdin already transferred"))?,
            self.stdout
                .take()
                .ok_or_else(|| io::Error::other("stdout already transferred"))?,
            self.stderr
                .take()
                .ok_or_else(|| io::Error::other("stderr already transferred"))?,
        ))
    }
}

pub(crate) fn spawn_suspended_descriptor(
    program: &Path,
    cwd: RawFd,
    argv: &[String],
    environment: &BTreeMap<String, String>,
) -> io::Result<DarwinSuspendedProcess> {
    super::darwin_spawn::spawn_suspended_descriptor(program, cwd, argv, environment)
}
