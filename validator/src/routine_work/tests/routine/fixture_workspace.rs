use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RoutineFixtureIdentity {
    device: u64,
    inode: u64,
}

pub(crate) struct RoutineFixtureClaim {
    path: PathBuf,
    directory: File,
    identity: RoutineFixtureIdentity,
    settled: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RoutineFixtureOwner {
    token: String,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum RoutineFixtureTeardown {
    Removed,
    RemovedAfterSubstitution { owned_path: PathBuf },
    AlreadyRemoved,
    Retained(RoutineFixtureResidue),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct RoutineFixtureResidue {
    pub(crate) claimed_path: PathBuf,
    pub(crate) owned_path: Option<PathBuf>,
    pub(crate) expected: RoutineFixtureIdentity,
    pub(crate) observed: Option<RoutineFixtureIdentity>,
    pub(crate) reason: &'static str,
}

pub(crate) fn claim_routine_fixture_root(
    label: &str,
    owner: &RoutineFixtureOwner,
) -> RoutineFixtureClaim {
    assert!(
        !label.is_empty()
            && label
                .bytes()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-'),
        "routine fixture label must be a semantic slug"
    );
    let parent = fixture_parent();
    fs::create_dir_all(&parent).unwrap();
    for _ in 0..64 {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).expect("routine fixture nonce unavailable");
        let candidate = parent.join(format!(
            "hul-routine-{label}-{}-{}-{}",
            std::process::id(),
            owner.token(),
            hex(&nonce)
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return RoutineFixtureClaim::open(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("routine fixture claim failed: {error}"),
        }
    }
    panic!("routine fixture claim collisions exhausted")
}

impl RoutineFixtureOwner {
    pub(crate) fn new() -> Self {
        let mut nonce = [0_u8; 16];
        getrandom::fill(&mut nonce).expect("routine fixture owner nonce unavailable");
        Self { token: hex(&nonce) }
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }
}

impl RoutineFixtureClaim {
    fn open(path: PathBuf) -> Self {
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&path)
            .unwrap();
        let identity = identity(&directory.metadata().unwrap());
        Self {
            path,
            directory,
            identity,
            settled: false,
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    // This pathname removal is permitted only after every fixture actor has stopped.
    // The open directory identifies a pre-teardown substitution before any removal.
    pub(crate) fn teardown_quiescent(&mut self) -> RoutineFixtureTeardown {
        if self.settled {
            return RoutineFixtureTeardown::AlreadyRemoved;
        }
        let Ok(owned_path) = descriptor_path(&self.directory, &self.path) else {
            return self.retained(None, None, "routine-fixture-descriptor-path-unavailable");
        };
        let observed = fs::symlink_metadata(&owned_path)
            .ok()
            .map(|value| identity(&value));
        if observed.map(RoutineFixtureIdentity::tuple) != Some(self.identity.tuple()) {
            return self.retained(
                Some(owned_path),
                observed,
                "routine-fixture-identity-mismatch",
            );
        }
        if fs::remove_dir_all(&owned_path).is_err() {
            return self.retained(
                Some(owned_path),
                observed,
                "routine-fixture-removal-incomplete",
            );
        }
        self.settled = true;
        if owned_path == self.path {
            RoutineFixtureTeardown::Removed
        } else {
            RoutineFixtureTeardown::RemovedAfterSubstitution { owned_path }
        }
    }

    fn retained(
        &self,
        owned_path: Option<PathBuf>,
        observed: Option<RoutineFixtureIdentity>,
        reason: &'static str,
    ) -> RoutineFixtureTeardown {
        RoutineFixtureTeardown::Retained(RoutineFixtureResidue {
            claimed_path: self.path.clone(),
            owned_path,
            expected: self.identity,
            observed,
            reason,
        })
    }
}

impl RoutineFixtureTeardown {
    pub(crate) fn assert_removed(self) {
        match self {
            Self::Removed | Self::AlreadyRemoved => {}
            Self::RemovedAfterSubstitution { owned_path } => {
                assert!(!owned_path.exists(), "owned substituted fixture remained")
            }
            Self::Retained(residue) => panic!(
                "routine fixture retained: reason={} claimed={:?} owned={:?} expected={:?} observed={:?}",
                residue.reason,
                residue.claimed_path,
                residue.owned_path,
                residue.expected.tuple(),
                residue.observed.map(RoutineFixtureIdentity::tuple)
            ),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

pub(crate) fn fixture_parent() -> PathBuf {
    let worktree = std::env::var_os("CODEX_WORKTREE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CODEX_WORKTREE_ROOT is required"));
    fs::canonicalize(worktree)
        .expect("configured worktree root unavailable")
        .join("target/routine-work-contract-fixtures")
}

fn identity(metadata: &fs::Metadata) -> RoutineFixtureIdentity {
    RoutineFixtureIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

impl RoutineFixtureIdentity {
    fn tuple(self) -> (u64, u64) {
        (self.device, self.inode)
    }
}

#[cfg(target_os = "macos")]
fn descriptor_path(directory: &File, _original: &Path) -> std::io::Result<PathBuf> {
    use std::ffi::OsStr;
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStrExt;

    let mut bytes = vec![0_u8; libc::PATH_MAX as usize];
    if unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_GETPATH, bytes.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    Ok(PathBuf::from(OsStr::from_bytes(&bytes[..end])))
}

#[cfg(not(target_os = "macos"))]
fn descriptor_path(_directory: &File, original: &Path) -> std::io::Result<PathBuf> {
    Ok(original.to_path_buf())
}
