use super::*;

pub(crate) const MAX_OUTPUT: usize = 1024 * 1024;
pub(crate) const MAX_EXECUTABLE_BYTES: usize = 64 * 1024 * 1024;
pub(crate) const MAX_SHELL_SOURCE_BYTES: usize = 64 * 1024;
#[cfg(unix)]
pub(crate) const PROTECTED_NATIVE_PATHS: [&str; 2] = ["/usr/bin/false", "/usr/bin/true"];
#[cfg(unix)]
pub(crate) const POSIX_SHELL_PATH: &str = "/bin/sh";

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExecutableIdentity {
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) length: u64,
    pub(crate) link_count: u64,
    pub(crate) user_id: u32,
    pub(crate) group_id: u32,
    pub(crate) mode: u32,
    pub(crate) modified_seconds: i64,
    pub(crate) modified_nanos: i64,
    pub(crate) changed_seconds: i64,
    pub(crate) changed_nanos: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PinnedExecutableKind {
    ProtectedNative,
    PosixShellSource,
    #[cfg(test)]
    TestNativeSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExecutableIssuancePolicy {
    Production,
    ShellSubstrate,
    #[cfg(test)]
    TestNativeSnapshot,
}

pub(crate) struct FixtureCaptureRequest {
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<OsString>,
    pub(crate) output_limit: usize,
    pub(crate) required_output: Vec<u8>,
    pub(crate) binding: FixtureExecutionBinding,
}

/// Crate-internal permit binding a pinned invocation to immutable fixture
/// metadata.  Public `CommandSpec` cannot construct or replace this permit.
pub(crate) struct FixtureCaptureAdapter {
    pub(crate) fixture_id: String,
    pub(crate) fixture_digest: String,
    pub(crate) executable: PathBuf,
    pub(crate) executable_digest: String,
    pub(crate) executable_bytes: Arc<[u8]>,
    pub(crate) executable_kind: PinnedExecutableKind,
    #[cfg(unix)]
    pub(crate) executable_identity: ExecutableIdentity,
    pub(crate) arguments: Vec<OsString>,
    pub(crate) output_limit: usize,
    pub(crate) interrupt: Arc<AtomicBool>,
    pub(crate) required_output: Vec<u8>,
    pub(crate) binding: FixtureExecutionBinding,
    pub(crate) artifact_name: Option<String>,
}
