use super::selection::SelectedCodexExecutable;
use crate::distribution::DistributionError;
use crate::distribution::error::{DistributionErrorId, error};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

type Observation = (PathBuf, String, String, Vec<u8>, Vec<u8>, Vec<u8>);

impl SelectedCodexExecutable {
    /// Runs the exact supported Codex help/listing surface inside a macOS
    /// sandbox that denies every filesystem write, clone/link, network
    /// operation, and descendant process. HOME, CODEX_HOME, and the working
    /// directory are descriptor-retained around all three observations.
    pub(crate) fn observe_supported_read_only(
        path: &Path,
        expected_sha256: &str,
        home: &Path,
        working_directory: &Path,
    ) -> Result<Observation, DistributionError> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (path, expected_sha256, home, working_directory);
            return Err(error(DistributionErrorId::CapabilityMismatch));
        }

        #[cfg(target_os = "macos")]
        {
            use std::os::fd::AsRawFd;

            if !valid_prefixed_sha256(expected_sha256) {
                return Err(error(DistributionErrorId::ProvenanceMismatch));
            }
            let home = RetainedDirectory::open_exact(home)?;
            let codex_home = RetainedDirectory::open_exact(&home.path.join(".codex"))?;
            let working_directory = RetainedDirectory::open_exact(working_directory)?;
            home.revalidate()?;
            codex_home.revalidate()?;
            working_directory.revalidate()?;

            let executable =
                Self::pin_path(path).map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
            if executable.content_sha256() != expected_sha256 {
                let _ = executable.finalize();
                return Err(error(DistributionErrorId::ProvenanceMismatch));
            }
            let executable_path = executable.canonical_path().to_path_buf();
            let executable_binding_sha256 = executable
                .binding_sha256()
                .map_err(|_| error(DistributionErrorId::ProvenanceMismatch))?;
            let capability = super::super::transaction_policy::current_capability()
                .map_err(|_| error(DistributionErrorId::CapabilityMismatch))?;
            let policy =
                super::super::executor::HostEffectExecutionPolicy::strict_personal_codex_home(
                    30_000, &home.path,
                )
                .map_err(|_| error(DistributionErrorId::CapabilityMismatch))?;
            let cancellation = super::super::executor::HostEffectCancellation::default();
            let environment = vec![
                (
                    "CODEX_HOME".to_owned(),
                    codex_home.path.display().to_string(),
                ),
                ("HOME".to_owned(), home.path.display().to_string()),
            ];

            let observed = (|| {
                let plugin_add_help = execute_command(
                    &executable,
                    &capability,
                    &policy,
                    &cancellation,
                    working_directory.file.as_raw_fd(),
                    &environment,
                    &["plugin", "add", "--help"],
                )?;
                revalidate_directories(&home, &codex_home, &working_directory)?;
                let plugin_list_json = execute_command(
                    &executable,
                    &capability,
                    &policy,
                    &cancellation,
                    working_directory.file.as_raw_fd(),
                    &environment,
                    &["plugin", "list", "--json"],
                )?;
                revalidate_directories(&home, &codex_home, &working_directory)?;
                let marketplace_list_json = execute_command(
                    &executable,
                    &capability,
                    &policy,
                    &cancellation,
                    working_directory.file.as_raw_fd(),
                    &environment,
                    &["plugin", "marketplace", "list", "--json"],
                )?;
                revalidate_directories(&home, &codex_home, &working_directory)?;
                Ok::<_, DistributionError>((
                    plugin_add_help,
                    plugin_list_json,
                    marketplace_list_json,
                ))
            })();
            let finalized = executable.finalize();
            let (plugin_add_help, plugin_list_json, marketplace_list_json) = observed?;
            finalized.map_err(|_| error(DistributionErrorId::ObjectChanged))?;
            Ok((
                executable_path,
                expected_sha256.to_owned(),
                executable_binding_sha256,
                plugin_add_help,
                plugin_list_json,
                marketplace_list_json,
            ))
        }
    }
}

#[cfg(target_os = "macos")]
fn execute_command(
    executable: &SelectedCodexExecutable,
    capability: &super::super::lifecycle::DescriptorExecutionCapability,
    policy: &super::super::executor::HostEffectExecutionPolicy,
    cancellation: &super::super::executor::HostEffectCancellation,
    cwd: std::os::fd::RawFd,
    environment: &[(String, String)],
    arguments: &[&str],
) -> Result<Vec<u8>, DistributionError> {
    let command = crate::distribution::HostCommand::from_untrusted_record(
        "codex".to_owned(),
        arguments.iter().map(|value| (*value).to_owned()).collect(),
        environment.to_vec(),
        30_000,
        1,
    );
    let capture = executable
        .execute_runtime(capability, &command, policy, cancellation, cwd)
        .map_err(|failure| {
            use super::super::executor::HostEffectExecutorErrorId;
            error(match failure.id {
                HostEffectExecutorErrorId::UnsupportedPlatform => {
                    DistributionErrorId::CapabilityMismatch
                }
                HostEffectExecutorErrorId::ProcessSpawnFailed => {
                    DistributionErrorId::ObjectUnavailable
                }
                HostEffectExecutorErrorId::OutputOverflow => DistributionErrorId::ObjectTooLarge,
                HostEffectExecutorErrorId::ExecutableMutation => DistributionErrorId::ObjectChanged,
                _ => DistributionErrorId::EffectFailed,
            })
        })?;
    if capture.exit_code() != 0 || !allowed_stderr(&capture.stderr) {
        return Err(error(DistributionErrorId::EffectFailed));
    }
    Ok(capture.stdout().to_vec())
}

#[cfg(target_os = "macos")]
fn allowed_stderr(stderr: &[u8]) -> bool {
    stderr.is_empty()
        || stderr
            == b"WARNING: proceeding, even though we could not create PATH aliases: Operation not permitted (os error 1)\n"
}

#[cfg(target_os = "macos")]
fn revalidate_directories(
    home: &RetainedDirectory,
    codex_home: &RetainedDirectory,
    working_directory: &RetainedDirectory,
) -> Result<(), DistributionError> {
    home.revalidate()?;
    codex_home.revalidate()?;
    working_directory.revalidate()
}

#[cfg(target_os = "macos")]
struct RetainedDirectory {
    path: PathBuf,
    file: File,
    device: u64,
    inode: u64,
    uid: u32,
    mode: u32,
}

#[cfg(target_os = "macos")]
impl RetainedDirectory {
    fn open_exact(path: &Path) -> Result<Self, DistributionError> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

        let canonical = path
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if canonical != path || !canonical.is_absolute() {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let named = fs::symlink_metadata(&canonical)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if named.file_type().is_symlink() || !named.is_dir() {
            return Err(error(DistributionErrorId::UnsafeObject));
        }
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
        let file = options
            .open(&canonical)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        let opened = file
            .metadata()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if opened.dev() != named.dev()
            || opened.ino() != named.ino()
            || opened.uid() != named.uid()
            || opened.mode() != named.mode()
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(Self {
            path: canonical,
            file,
            device: opened.dev(),
            inode: opened.ino(),
            uid: opened.uid(),
            mode: opened.mode(),
        })
    }

    fn revalidate(&self) -> Result<(), DistributionError> {
        use std::os::unix::fs::MetadataExt;

        let descriptor = self
            .file
            .metadata()
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        let named = fs::symlink_metadata(&self.path)
            .map_err(|_| error(DistributionErrorId::ObjectChanged))?;
        if named.file_type().is_symlink()
            || !named.is_dir()
            || descriptor.dev() != self.device
            || descriptor.ino() != self.inode
            || descriptor.uid() != self.uid
            || descriptor.mode() != self.mode
            || named.dev() != self.device
            || named.ino() != self.inode
            || named.uid() != self.uid
            || named.mode() != self.mode
            || self.path.canonicalize().ok().as_deref() != Some(self.path.as_path())
        {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }
}

fn valid_prefixed_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
