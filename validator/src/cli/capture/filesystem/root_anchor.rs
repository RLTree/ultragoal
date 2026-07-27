use super::*;

#[cfg(test)]
pub(crate) static TEST_ARTIFACT_PAUSE_MS: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
pub(crate) static TEST_ARTIFACT_PAUSED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn set_test_artifact_pause_ms(milliseconds: u64) {
    TEST_ARTIFACT_PAUSED.store(false, Ordering::SeqCst);
    TEST_ARTIFACT_PAUSE_MS.store(milliseconds, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_artifact_is_paused() -> bool {
    TEST_ARTIFACT_PAUSED.load(Ordering::SeqCst)
}

#[cfg(test)]
pub(crate) fn test_artifact_pause() {
    let milliseconds = TEST_ARTIFACT_PAUSE_MS.swap(0, Ordering::SeqCst);
    TEST_ARTIFACT_PAUSED.store(true, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(milliseconds));
}

#[cfg(not(test))]
pub(crate) fn test_artifact_pause() {}

pub(crate) struct RootAnchor {
    pub(crate) root: PathBuf,
    pub(crate) file: File,
    #[cfg(unix)]
    pub(crate) identity: DirectoryIdentity,
}

pub(crate) struct PinnedDirectory {
    pub(crate) file: File,
    pub(crate) relative: PathBuf,
    #[cfg(unix)]
    pub(crate) identity: DirectoryIdentity,
}

pub(crate) struct PinnedFile {
    pub(crate) file: File,
    pub(crate) relative: PathBuf,
    pub(crate) parent: PinnedDirectory,
    #[cfg(unix)]
    pub(crate) snapshot: Snapshot,
    #[cfg(unix)]
    pub(crate) parent_snapshot: Snapshot,
    pub(crate) initial_sha256: String,
    pub(crate) initial_bytes: Arc<[u8]>,
    pub(crate) read_limit: usize,
}

impl RootAnchor {
    pub fn new(context: &LiveContext) -> Result<Self, String> {
        validate_context_roots(context)?;
        context
            .revalidate()
            .map_err(|_| "live context failed pre-execution revalidation".to_owned())?;
        let recorded = context.worktree_root();
        let root = recorded
            .canonicalize()
            .map_err(|_| "worktree root cannot be canonicalized".to_owned())?;
        if root != recorded {
            return Err("worktree root is not canonical".to_owned());
        }
        let file = File::open(&root).map_err(|_| "worktree root cannot be opened".to_owned())?;
        let metadata = file
            .metadata()
            .map_err(|_| "worktree root metadata is unavailable".to_owned())?;
        if !metadata.is_dir() {
            return Err("worktree root is not a directory".to_owned());
        }
        #[cfg(not(unix))]
        return Err("descriptor-confined capture requires Unix".to_owned());
        #[cfg(unix)]
        {
            Ok(Self {
                root,
                file,
                identity: DirectoryIdentity::from(&metadata),
            })
        }
    }

    pub fn open_directory(&self, relative: &Path) -> Result<PinnedDirectory, String> {
        validate_relative(relative, "cwd")?;
        validate_public_path(relative, "cwd")?;
        self.validate()?;
        let mut directory = self
            .file
            .try_clone()
            .map_err(|_| "cwd open failed".to_owned())?;
        for component in relative.components() {
            match component {
                Component::CurDir => {}
                Component::Normal(name) => directory = open_dir_at(&directory, name)?,
                _ => return Err("cwd contains a forbidden path component".to_owned()),
            }
        }
        let metadata = directory
            .metadata()
            .map_err(|_| "cwd metadata is unavailable".to_owned())?;
        if !metadata.is_dir() {
            return Err("cwd is not a directory".to_owned());
        }
        Ok(PinnedDirectory {
            file: directory,
            relative: relative.to_path_buf(),
            #[cfg(unix)]
            identity: DirectoryIdentity::from(&metadata),
        })
    }

    pub fn open_regular(
        &self,
        relative: &Path,
        executable: bool,
        maximum_bytes: usize,
    ) -> Result<PinnedFile, String> {
        validate_relative(relative, "file path")?;
        validate_public_path(relative, "file path")?;
        self.validate()?;
        let (parent, name) = split_leaf(relative)?;
        let directory = self.open_directory(parent)?;
        let file = open_file_at(&directory.file, name, executable)?;
        let metadata = file
            .metadata()
            .map_err(|_| "file metadata is unavailable".to_owned())?;
        if !metadata.is_file() {
            return Err("path is not a regular file".to_owned());
        }
        #[cfg(unix)]
        if metadata.nlink() != 1 {
            return Err("hard-linked regular file is rejected".to_owned());
        }
        #[cfg(unix)]
        if executable && metadata.mode() & 0o111 == 0 {
            return Err("program is not executable".to_owned());
        }
        let (initial_bytes, initial_sha256) = read_descriptor(&file, maximum_bytes)?;
        #[cfg(unix)]
        let parent_snapshot = Snapshot::from(
            &directory
                .file
                .metadata()
                .map_err(|_| "file parent metadata is unavailable".to_owned())?,
        );
        Ok(PinnedFile {
            file,
            relative: relative.to_path_buf(),
            parent: directory,
            #[cfg(unix)]
            snapshot: Snapshot::from(&metadata),
            #[cfg(unix)]
            parent_snapshot,
            initial_sha256,
            initial_bytes: Arc::from(initial_bytes),
            read_limit: maximum_bytes,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        let canonical = self
            .root
            .canonicalize()
            .map_err(|_| "worktree root disappeared".to_owned())?;
        if canonical != self.root {
            return Err("worktree root identity changed".to_owned());
        }
        #[cfg(unix)]
        {
            let path = fs::metadata(&self.root)
                .map_err(|_| "worktree root metadata disappeared".to_owned())?;
            let descriptor = self
                .file
                .metadata()
                .map_err(|_| "worktree root descriptor failed".to_owned())?;
            if DirectoryIdentity::from(&path) != self.identity
                || DirectoryIdentity::from(&descriptor) != self.identity
            {
                return Err("worktree root changed during capture".to_owned());
            }
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

pub(crate) fn validate_context_roots(context: &LiveContext) -> Result<(), String> {
    if context.roots().repository_root != context.roots().worktree_root {
        return Err(
            "capture root unavailable: alternate repository/worktree roots require root-owned device/inode binding"
                .to_owned(),
        );
    }
    validate_public_path(context.worktree_root(), "worktree root")
}

impl PinnedDirectory {
    #[cfg(unix)]
    pub fn raw_fd(&self) -> std::os::fd::RawFd {
        use std::os::fd::AsRawFd;
        self.file.as_raw_fd()
    }

    pub fn validate(&self, root: &RootAnchor) -> Result<(), String> {
        let current = root.open_directory(&self.relative)?;
        #[cfg(unix)]
        if current.identity != self.identity
            || DirectoryIdentity::from(
                &self
                    .file
                    .metadata()
                    .map_err(|_| "cwd descriptor failed".to_owned())?,
            ) != self.identity
        {
            return Err("cwd changed during capture".to_owned());
        }
        Ok(())
    }
}
