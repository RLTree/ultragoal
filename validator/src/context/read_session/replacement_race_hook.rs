use super::*;

#[cfg(test)]
pub(crate) static TEST_PAUSE_BEFORE_OPEN_MS: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
pub(crate) static TEST_PAUSE_AFTER_FIRST_READ_MS: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub(crate) fn set_test_pause_before_open(milliseconds: u64) {
    TEST_PAUSE_BEFORE_OPEN_MS.store(milliseconds, Ordering::SeqCst);
}

#[cfg(test)]
pub(crate) fn set_test_pause_after_first_read(milliseconds: u64) {
    TEST_PAUSE_AFTER_FIRST_READ_MS.store(milliseconds, Ordering::SeqCst);
}

pub(crate) struct ReadSession {
    pub(crate) context: LiveContext,
    pub(crate) root: PathBuf,
    pub(crate) root_identity: Option<(u64, u64)>,
    pub(crate) directories: RefCell<BTreeMap<PathBuf, File>>,
    pub(in crate::context) directory_identities: RefCell<BTreeMap<PathBuf, FileSnapshot>>,
    pub(crate) observations: ObservationSet,
    pub(crate) bytes_read: Cell<u64>,
    pub(crate) entries_visited: Cell<u64>,
}

pub(crate) const MAX_READ_SESSION_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_READ_SESSION_ENTRIES: u64 = 250_000;

impl LiveContext {
    pub(crate) fn begin_read_session(&self) -> Result<ReadSession, ContextError> {
        self.revalidate()?;
        let root = self.worktree_root().to_path_buf();
        let root_file = File::open(&root).map_err(|error| io_error(&root, error))?;
        let metadata = root_file
            .metadata()
            .map_err(|error| io_error(&root, error))?;
        if !metadata.is_dir() {
            return Err(ContextError::PathDenied(format!(
                "read-session root is not a directory: {}",
                root.display()
            )));
        }
        #[cfg(unix)]
        let root_identity = Some((metadata.dev(), metadata.ino()));
        #[cfg(not(unix))]
        let root_identity = None;
        let mut directories = BTreeMap::new();
        directories.insert(root.clone(), root_file);
        let mut directory_identities = BTreeMap::new();
        directory_identities.insert(root.clone(), snapshot(&metadata));
        Ok(ReadSession {
            context: self.clone(),
            root,
            root_identity,
            directories: RefCell::new(directories),
            directory_identities: RefCell::new(directory_identities),
            observations: ObservationSet::new(),
            bytes_read: Cell::new(0),
            entries_visited: Cell::new(0),
        })
    }
}
