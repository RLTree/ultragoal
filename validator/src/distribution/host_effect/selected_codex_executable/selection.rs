use super::super::{
    HostEffectLedgerError, HostEffectLedgerErrorId, MAX_PINNED_EXECUTABLE_BYTES, invalid_record,
    ledger_io, read_at_retry, same_executable_object, tampered,
};
use super::identity::SelectedCodexExecutableIdentity;
use super::immutable_launch::ImmutableExecutable;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

pub(crate) struct SelectedCodexExecutable {
    file: File,
    launch: ImmutableExecutable,
    identity: SelectedCodexExecutableIdentity,
}

impl SelectedCodexExecutable {
    fn pin(path: &Path) -> Result<Self, HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let canonical = fs::canonicalize(path).map_err(|_| ledger_io())?;
            if !canonical.is_absolute() {
                return Err(invalid_record());
            }
            let mut options = OpenOptions::new();
            options
                .read(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
            let file = options.open(&canonical).map_err(|_| ledger_io())?;
            let identity = capture_identity(&file, &canonical)?;
            let launch = ImmutableExecutable::stage(
                &file,
                identity.mode,
                identity.size,
                &identity.content_sha256,
            )?;
            Ok(Self {
                file,
                launch,
                identity,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(invalid_record())
        }
    }

    pub(super) fn pin_path(path: &Path) -> Result<Self, HostEffectLedgerError> {
        Self::pin(path)
    }

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    pub(super) fn launch_file(&self) -> &File {
        self.launch.file()
    }

    pub(in crate::distribution::host_effect) fn binding_sha256(
        &self,
    ) -> Result<String, HostEffectLedgerError> {
        self.identity.binding_sha256()
    }

    pub(in crate::distribution::host_effect) fn content_sha256(&self) -> &str {
        &self.identity.content_sha256
    }

    pub(crate) fn observe_personal_plugin(
        &self,
        home: &Path,
        install_plan_sha256: &str,
        expected_content_sha256: &str,
        cancellation: &super::super::executor::HostEffectCancellation,
        lease_fd: std::os::fd::RawFd,
        codex_home_fd: std::os::fd::RawFd,
    ) -> Result<(PathBuf, Vec<u8>, Vec<u8>), &'static str> {
        if !exact_digest(install_plan_sha256) || self.content_sha256() != expected_content_sha256 {
            return Err("selected Codex executable identity changed");
        }
        let home = canonical_personal_home(home)?;
        let cwd = open_personal_home(&home)?;
        revalidate_personal_codex_home(&cwd, codex_home_fd)?;
        let policy = super::super::executor::HostEffectExecutionPolicy::strict_personal_codex_home(
            30_000, &home,
        )
        .map_err(|_| "personal Codex execution policy unavailable")?;
        let environment = personal_environment(&home);
        let marketplace = crate::distribution::HostCommand::from_untrusted_record(
            "codex".to_owned(),
            ["plugin", "marketplace", "list", "--json"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            environment.clone(),
            30_000,
            1,
        );
        let plugins = crate::distribution::HostCommand::from_untrusted_record(
            "codex".to_owned(),
            ["plugin", "list", "--json"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            environment,
            30_000,
            1,
        );
        let capability = super::super::transaction_policy::current_capability()
            .map_err(|_| "personal Codex execution capability unavailable")?;
        let marketplace = self
            .execute_personal(
                &capability,
                &marketplace,
                &policy,
                cancellation,
                cwd.as_raw_fd(),
                lease_fd,
                &home.join(".codex"),
            )
            .map_err(|_| "pinned Codex marketplace observation failed")?;
        if marketplace.exit_code() != 0 {
            return Err("pinned Codex marketplace observation returned failure");
        }
        let plugins = self
            .execute_personal(
                &capability,
                &plugins,
                &policy,
                cancellation,
                cwd.as_raw_fd(),
                lease_fd,
                &home.join(".codex"),
            )
            .map_err(|_| "pinned Codex plugin observation failed")?;
        if plugins.exit_code() != 0 {
            return Err("pinned Codex plugin observation returned failure");
        }
        Ok((
            self.identity.canonical_path().to_path_buf(),
            plugins.stdout().to_vec(),
            marketplace.stdout().to_vec(),
        ))
    }

    pub(crate) fn install_personal_plugin(
        &self,
        package: &crate::distribution::PackageIdentity,
        marketplace: &str,
        home: &Path,
        install_plan_sha256: &str,
        expected_content_sha256: &str,
        cancellation: &super::super::executor::HostEffectCancellation,
        lease_fd: std::os::fd::RawFd,
        codex_home_fd: std::os::fd::RawFd,
    ) -> Result<(), &'static str> {
        if !exact_digest(install_plan_sha256) || self.content_sha256() != expected_content_sha256 {
            return Err("selected Codex executable identity changed");
        }
        let home = canonical_personal_home(home)?;
        let cwd = open_personal_home(&home)?;
        revalidate_personal_codex_home(&cwd, codex_home_fd)?;
        let plan = crate::distribution::HostCommandPlan::personal_install_in_codex_home(
            package,
            marketplace,
            &home,
        )
        .map_err(|_| "personal Codex command plan unavailable")?;
        let command = plan
            .commands()
            .first()
            .ok_or("personal Codex command plan is empty")?;
        let policy = super::super::executor::HostEffectExecutionPolicy::strict_personal_codex_home(
            30_000, &home,
        )
        .map_err(|_| "personal Codex execution policy unavailable")?;
        let capability = super::super::transaction_policy::current_capability()
            .map_err(|_| "personal Codex execution capability unavailable")?;
        let capture = self
            .execute_personal(
                &capability,
                command,
                &policy,
                cancellation,
                cwd.as_raw_fd(),
                lease_fd,
                &home.join(".codex"),
            )
            .map_err(|_| "pinned Codex plugin installation failed")?;
        (capture.exit_code() == 0)
            .then_some(())
            .ok_or("pinned Codex plugin installation returned failure")
    }

    pub(crate) fn reinstall_personal_plugin(
        &self,
        prior: &crate::plugin_product::lifecycle::PriorInstalledAuthority,
        marketplace: &str,
        home: &Path,
        install_plan_sha256: &str,
        expected_content_sha256: &str,
        cancellation: &super::super::executor::HostEffectCancellation,
        lease_fd: std::os::fd::RawFd,
        codex_home_fd: std::os::fd::RawFd,
    ) -> Result<(), &'static str> {
        prior
            .validate()
            .map_err(|_| "prior installed authority is invalid")?;
        if marketplace != "local-harness-plugins"
            || !exact_digest(install_plan_sha256)
            || self.content_sha256() != expected_content_sha256
        {
            return Err("personal reinstall authority changed");
        }
        let home = canonical_personal_home(home)?;
        let cwd = open_personal_home(&home)?;
        revalidate_personal_codex_home(&cwd, codex_home_fd)?;
        let command = crate::distribution::HostCommand::from_untrusted_record(
            "codex".to_owned(),
            vec![
                "plugin".to_owned(),
                "add".to_owned(),
                "harness-ultragoal@local-harness-plugins".to_owned(),
            ],
            personal_environment(&home),
            30_000,
            1,
        );
        let policy = super::super::executor::HostEffectExecutionPolicy::strict_personal_codex_home(
            30_000, &home,
        )
        .map_err(|_| "personal Codex execution policy unavailable")?;
        let capability = super::super::transaction_policy::current_capability()
            .map_err(|_| "personal Codex execution capability unavailable")?;
        let capture = self
            .execute_personal(
                &capability,
                &command,
                &policy,
                cancellation,
                cwd.as_raw_fd(),
                lease_fd,
                &home.join(".codex"),
            )
            .map_err(|_| "pinned Codex prior reinstallation failed")?;
        (capture.exit_code() == 0)
            .then_some(())
            .ok_or("pinned Codex prior reinstallation returned failure")
    }

    pub(crate) fn finalize_personal(self) -> Result<(), &'static str> {
        self.finalize()
            .map_err(|_| "pinned Codex executable cleanup failed")
    }

    #[cfg(test)]
    pub(in crate::distribution::host_effect) fn duplicate(
        &self,
    ) -> Result<Self, HostEffectLedgerError> {
        Ok(Self {
            file: self.file.try_clone().map_err(|_| ledger_io())?,
            launch: self.launch.duplicate()?,
            identity: self.identity.clone(),
        })
    }

    pub(in crate::distribution::host_effect) fn finalize(
        self,
    ) -> Result<(), HostEffectLedgerError> {
        let Self {
            file,
            launch,
            identity: _,
        } = self;
        drop(file);
        launch.finalize()
    }

    pub(in crate::distribution::host_effect) fn revalidate(
        &self,
    ) -> Result<(), HostEffectLedgerError> {
        #[cfg(unix)]
        {
            let current = capture_identity(&self.file, Path::new(&self.identity.canonical_path))
                .map_err(|_| HostEffectLedgerError::new(HostEffectLedgerErrorId::Tampered))?;
            if current != self.identity {
                return Err(HostEffectLedgerError::new(
                    HostEffectLedgerErrorId::Tampered,
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err(invalid_record())
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn launch_path(&self) -> &Path {
        self.launch.path()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn revalidate_launch(&self) -> Result<(), HostEffectLedgerError> {
        self.launch.revalidate()
    }

    pub(in crate::distribution::host_effect) fn execute(
        &self,
        capability: &super::super::lifecycle::DescriptorExecutionCapability,
        command: &super::super::super::HostCommand,
        policy: &super::super::executor::HostEffectExecutionPolicy,
        cancellation: &super::super::executor::HostEffectCancellation,
        cwd: std::os::fd::RawFd,
    ) -> Result<super::super::executor::CommandCapture, super::super::executor::BackendFailure>
    {
        self.revalidate().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        #[cfg(target_os = "macos")]
        self.revalidate_launch().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        super::execution::execute(self, capability, command, policy, cancellation, cwd)
    }

    pub(in crate::distribution::host_effect) fn execute_runtime(
        &self,
        capability: &super::super::lifecycle::DescriptorExecutionCapability,
        command: &super::super::super::HostCommand,
        policy: &super::super::executor::HostEffectExecutionPolicy,
        cancellation: &super::super::executor::HostEffectCancellation,
        cwd: std::os::fd::RawFd,
    ) -> Result<super::super::executor::CommandCapture, super::super::executor::BackendFailure>
    {
        self.revalidate().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        #[cfg(target_os = "macos")]
        self.revalidate_launch().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        super::execution::execute_runtime(self, capability, command, policy, cancellation, cwd)
    }

    fn execute_personal(
        &self,
        capability: &super::super::lifecycle::DescriptorExecutionCapability,
        command: &super::super::super::HostCommand,
        policy: &super::super::executor::HostEffectExecutionPolicy,
        cancellation: &super::super::executor::HostEffectCancellation,
        cwd: std::os::fd::RawFd,
        lease_fd: std::os::fd::RawFd,
        codex_home: &Path,
    ) -> Result<super::super::executor::CommandCapture, super::super::executor::BackendFailure>
    {
        self.revalidate().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        #[cfg(target_os = "macos")]
        self.revalidate_launch().map_err(|_| {
            super::super::executor::BackendFailure::before_start(
                super::super::executor::HostEffectExecutorErrorId::ExecutableMutation,
            )
        })?;
        super::execution::execute_personal(
            self,
            capability,
            command,
            policy,
            cancellation,
            cwd,
            lease_fd,
            codex_home,
        )
    }
}

fn canonical_personal_home(home: &Path) -> Result<PathBuf, &'static str> {
    let canonical = home
        .canonicalize()
        .map_err(|_| "personal home unavailable")?;
    if canonical != home || !canonical.is_absolute() {
        return Err("personal home binding changed");
    }
    Ok(canonical)
}

fn open_personal_home(home: &Path) -> Result<File, &'static str> {
    use std::os::unix::fs::OpenOptionsExt;
    let metadata = fs::symlink_metadata(home).map_err(|_| "personal home unavailable")?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("personal home is not a directory");
    }
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
    let file = options
        .open(home)
        .map_err(|_| "personal home descriptor unavailable")?;
    let opened = file
        .metadata()
        .map_err(|_| "personal home descriptor unavailable")?;
    if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
        return Err("personal home binding changed");
    }
    Ok(file)
}

fn revalidate_personal_codex_home(
    home: &File,
    retained_codex_home: std::os::fd::RawFd,
) -> Result<(), &'static str> {
    let mut named = std::mem::MaybeUninit::<libc::stat>::uninit();
    let mut retained = std::mem::MaybeUninit::<libc::stat>::uninit();
    let named_result = unsafe {
        libc::fstatat(
            home.as_raw_fd(),
            c".codex".as_ptr(),
            named.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    let retained_result = unsafe { libc::fstat(retained_codex_home, retained.as_mut_ptr()) };
    if named_result != 0 || retained_result != 0 {
        return Err("personal Codex home authority is unavailable");
    }
    let named = unsafe { named.assume_init() };
    let retained = unsafe { retained.assume_init() };
    if named.st_mode & libc::S_IFMT != libc::S_IFDIR
        || retained.st_mode & libc::S_IFMT != libc::S_IFDIR
        || named.st_dev != retained.st_dev
        || named.st_ino != retained.st_ino
        || named.st_uid != retained.st_uid
        || named.st_mode != retained.st_mode
    {
        return Err("personal Codex home authority changed before child spawn");
    }
    Ok(())
}

fn personal_environment(home: &Path) -> Vec<(String, String)> {
    let codex_home = home.join(".codex").display().to_string();
    let home = home.display().to_string();
    vec![
        ("CODEX_HOME".to_owned(), codex_home),
        ("HOME".to_owned(), home),
    ]
}

fn exact_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(super) fn resolve_from_path(
    paths: impl IntoIterator<Item = PathBuf>,
    program: &str,
) -> Result<SelectedCodexExecutable, HostEffectLedgerError> {
    for directory in paths {
        if !directory.is_absolute() {
            continue;
        }
        let candidate = directory.join(program);
        if let Ok(executable) = SelectedCodexExecutable::pin(&candidate) {
            return Ok(executable);
        }
    }
    Err(ledger_io())
}

pub(super) fn pin_path(path: &Path) -> Result<SelectedCodexExecutable, HostEffectLedgerError> {
    SelectedCodexExecutable::pin_path(path)
}

#[cfg(unix)]
fn capture_identity(
    file: &File,
    canonical_path: &Path,
) -> Result<SelectedCodexExecutableIdentity, HostEffectLedgerError> {
    let path_before = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    let descriptor_before = file.metadata().map_err(|_| ledger_io())?;
    validate_executable_metadata(&path_before)?;
    validate_executable_metadata(&descriptor_before)?;
    if !same_executable_object(&path_before, &descriptor_before) {
        return Err(tampered());
    }

    let mut hasher = Sha256::new();
    let mut offset = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    while offset < descriptor_before.len() {
        let remaining = (descriptor_before.len() - offset).min(buffer.len() as u64) as usize;
        let count = read_at_retry(file, &mut buffer[..remaining], offset)?;
        if count == 0 {
            return Err(tampered());
        }
        hasher.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(invalid_record)?;
    }
    let mut extra = [0_u8; 1];
    if read_at_retry(file, &mut extra, descriptor_before.len())? != 0 {
        return Err(tampered());
    }

    let descriptor_after = file.metadata().map_err(|_| ledger_io())?;
    let path_after = fs::symlink_metadata(canonical_path).map_err(|_| ledger_io())?;
    validate_executable_metadata(&descriptor_after)?;
    validate_executable_metadata(&path_after)?;
    if !same_executable_object(&descriptor_before, &descriptor_after)
        || !same_executable_object(&descriptor_after, &path_after)
        || fs::canonicalize(canonical_path).map_err(|_| ledger_io())? != canonical_path
    {
        return Err(tampered());
    }
    let canonical_path = canonical_path
        .to_str()
        .ok_or_else(invalid_record)?
        .to_owned();
    Ok(SelectedCodexExecutableIdentity {
        canonical_path,
        content_sha256: format!("sha256:{:x}", hasher.finalize()),
        device: descriptor_after.dev(),
        inode: descriptor_after.ino(),
        mode: descriptor_after.mode(),
        uid: descriptor_after.uid(),
        gid: descriptor_after.gid(),
        hard_links: descriptor_after.nlink(),
        size: descriptor_after.len(),
        modified_seconds: descriptor_after.mtime(),
        modified_nanoseconds: descriptor_after.mtime_nsec(),
        changed_seconds: descriptor_after.ctime(),
        changed_nanoseconds: descriptor_after.ctime_nsec(),
    })
}

#[cfg(unix)]
fn validate_executable_metadata(metadata: &fs::Metadata) -> Result<(), HostEffectLedgerError> {
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_PINNED_EXECUTABLE_BYTES
        || metadata.nlink() != 1
        || metadata.mode() & 0o111 == 0
    {
        return Err(invalid_record());
    }
    Ok(())
}
