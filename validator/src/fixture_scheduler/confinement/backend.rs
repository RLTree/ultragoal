use super::ConfinementPolicy;
use crate::fixture_scheduler::FixtureScheduleError;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

const DARWIN_SANDBOX: &str = "/usr/bin/sandbox-exec";

pub(crate) struct ConfinementPlan {
    backend: PathBuf,
    profile: String,
    policy: ConfinementPolicy,
    lease_root: PathBuf,
    #[cfg(target_os = "macos")]
    lease_identity: RootIdentity,
    address_space_limit: u64,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RootIdentity {
    device: u64,
    inode: u64,
}

impl ConfinementPlan {
    pub(crate) fn prepare(
        policy: &ConfinementPolicy,
        lease_root: &Path,
    ) -> Result<Self, FixtureScheduleError> {
        Self::prepare_with_backend(policy, lease_root, Path::new(DARWIN_SANDBOX))
    }

    pub(crate) fn prepare_with_backend(
        policy: &ConfinementPolicy,
        lease_root: &Path,
        backend: &Path,
    ) -> Result<Self, FixtureScheduleError> {
        policy.validate()?;
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (lease_root, backend);
            return Err(FixtureScheduleError::Integrity(
                "fixture confinement backend is unsupported on this host".to_owned(),
            ));
        }
        #[cfg(target_os = "macos")]
        {
            if backend != Path::new(DARWIN_SANDBOX) {
                return Err(FixtureScheduleError::Integrity(
                    "fixture confinement backend is not the fixed Darwin substrate".to_owned(),
                ));
            }
            protected_executable(backend)?;
            let root = lease_root
                .canonicalize()
                .map_err(FixtureScheduleError::Io)?;
            let metadata = fs::symlink_metadata(lease_root)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(FixtureScheduleError::Integrity(
                    "fixture confinement lease root identity is unsafe".to_owned(),
                ));
            }
            let lease_identity = RootIdentity::from(&metadata);
            let canonical_identity = RootIdentity::from(&fs::metadata(&root)?);
            if lease_identity != canonical_identity {
                return Err(FixtureScheduleError::Integrity(
                    "fixture confinement lease root identity changed during preparation".to_owned(),
                ));
            }
            let address_space_limit = darwin_address_space_limit(policy.address_space_bytes)?;
            Ok(Self {
                backend: backend.to_path_buf(),
                profile: profile(&root)?,
                policy: policy.clone(),
                lease_root: root,
                lease_identity,
                address_space_limit,
            })
        }
    }

    pub(crate) fn command(
        &self,
        executable: &Path,
        arguments: &[OsString],
        cwd: &Path,
        environment: &BTreeMap<String, String>,
    ) -> Result<Command, FixtureScheduleError> {
        #[cfg(target_os = "macos")]
        self.validate_lease_root(cwd)?;
        let mut command = Command::new(&self.backend);
        command
            .arg("-p")
            .arg(&self.profile)
            .arg(executable)
            .args(arguments)
            .current_dir(&self.lease_root)
            .env_clear()
            .envs(environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            let policy = self.policy.clone();
            let address_space_limit = self.address_space_limit;
            unsafe {
                command.pre_exec(move || apply_limits(&policy, address_space_limit));
            }
        }
        #[cfg(not(unix))]
        return Err(FixtureScheduleError::Integrity(
            "fixture confinement requires Unix pre-exec limits".to_owned(),
        ));
        Ok(command)
    }

    pub(crate) fn address_space_limit(&self) -> u64 {
        self.address_space_limit
    }

    #[cfg(target_os = "macos")]
    fn validate_lease_root(&self, supplied_root: &Path) -> Result<(), FixtureScheduleError> {
        let metadata = fs::symlink_metadata(supplied_root)?;
        let canonical = supplied_root.canonicalize()?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || canonical != self.lease_root
            || RootIdentity::from(&metadata) != self.lease_identity
        {
            return Err(FixtureScheduleError::Integrity(
                "fixture confinement lease root identity changed before execution".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl RootIdentity {
    fn from(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

#[cfg(target_os = "macos")]
fn profile(root: &Path) -> Result<String, FixtureScheduleError> {
    let root = root.to_str().ok_or_else(|| {
        FixtureScheduleError::InvalidMetadata("fixture lease root is not UTF-8".to_owned())
    })?;
    let escaped = root.replace('\\', "\\\\").replace('"', "\\\"");
    Ok(format!(
        "(version 1)\n(allow default)\n(deny process-fork)\n(deny network*)\n(deny file-write*)\n(allow file-write* (subpath \"{escaped}\"))\n"
    ))
}

#[cfg(target_os = "macos")]
fn protected_executable(path: &Path) -> Result<(), FixtureScheduleError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.uid() != 0
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
        || metadata.mode() & 0o022 != 0
    {
        return Err(FixtureScheduleError::Integrity(
            "fixture confinement backend identity is unsafe".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn darwin_address_space_limit(budget: u64) -> Result<u64, FixtureScheduleError> {
    unsafe extern "C" {
        #[link_name = "mach_task_self_"]
        static MACH_TASK_SELF_PORT: libc::mach_port_t;
    }
    let mut info = std::mem::MaybeUninit::<libc::mach_task_basic_info>::zeroed();
    let mut count = libc::MACH_TASK_BASIC_INFO_COUNT;
    // SAFETY: this copies the process-global Mach task port value; no reference
    // to the mutable foreign static is created or retained.
    let task = unsafe { MACH_TASK_SELF_PORT };
    let status = unsafe {
        libc::task_info(
            task,
            libc::MACH_TASK_BASIC_INFO,
            info.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if status != libc::KERN_SUCCESS || count != libc::MACH_TASK_BASIC_INFO_COUNT {
        return Err(FixtureScheduleError::Integrity(
            "fixture confinement could not observe the Darwin address-space baseline".to_owned(),
        ));
    }
    let info = unsafe { info.assume_init() };
    let baseline = unsafe { std::ptr::addr_of!(info.virtual_size).read_unaligned() };
    let limit = baseline.checked_add(budget).ok_or_else(|| {
        FixtureScheduleError::InvalidMetadata(
            "fixture confinement address-space budget overflows the host limit".to_owned(),
        )
    })?;
    if limit >= libc::RLIM_INFINITY {
        return Err(FixtureScheduleError::InvalidMetadata(
            "fixture confinement address-space limit is not finite".to_owned(),
        ));
    }
    Ok(limit)
}

#[cfg(unix)]
fn apply_limits(policy: &ConfinementPolicy, address_space_limit: u64) -> std::io::Result<()> {
    set_limit(libc::RLIMIT_CPU, policy.cpu_seconds)?;
    set_limit(libc::RLIMIT_AS, address_space_limit)?;
    set_limit(libc::RLIMIT_FSIZE, policy.maximum_file_bytes)?;
    if unsafe { libc::setpgid(0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn set_limit(resource: libc::c_int, value: u64) -> std::io::Result<()> {
    let limit = libc::rlimit {
        rlim_cur: value as libc::rlim_t,
        rlim_max: value as libc::rlim_t,
    };
    if unsafe { libc::setrlimit(resource as _, &limit) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
