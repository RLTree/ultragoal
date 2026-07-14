use super::*;

impl LocalEffects {
    pub(crate) fn open(
        root: impl AsRef<Path>,
        unix_modes: BTreeMap<String, u32>,
    ) -> Result<Self, FitError> {
        if unix_modes.iter().any(|(path, mode)| {
            CanonicalPath::parse(path).is_err() || !matches!(mode, 0o644 | 0o755)
        }) {
            return Err(error(FitErrorId::InvalidSpec));
        }
        let path = root.as_ref().to_path_buf();
        let path_metadata =
            fs::symlink_metadata(&path).map_err(|_| error(FitErrorId::UnsafeObject))?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(error(FitErrorId::UnsafeObject));
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let root_file = options
            .open(&path)
            .map_err(|_| error(FitErrorId::UnsafeObject))?;
        let metadata = root_file
            .metadata()
            .map_err(|_| error(FitErrorId::ReadFailed))?;
        let root_identity = object_identity(&metadata);
        if !metadata.is_dir() || root_identity != object_identity(&path_metadata) {
            return Err(error(FitErrorId::UnsafeObject));
        }
        let canonical = fs::canonicalize(&path).map_err(|_| error(FitErrorId::ReadFailed))?;
        let reader = LocalRepository::open(&path)?;
        let mut value = Self {
            path,
            canonical,
            root: root_file,
            root_identity,
            binding: String::new(),
            reader,
            mutation_lease: false,
            unix_modes,
            rollback_modes: BTreeMap::new(),
            created_directories: BTreeMap::new(),
            #[cfg(test)]
            fail_calls: BTreeSet::new(),
            #[cfg(test)]
            compare_calls: 0,
            #[cfg(test)]
            before_linearize: None,
            #[cfg(test)]
            after_replace_swap: None,
            #[cfg(test)]
            before_quarantine_move: None,
            #[cfg(test)]
            after_directory_publish: None,
            #[cfg(test)]
            before_directory_quarantine_move: None,
        };
        value.binding = value.reader.root_binding()?;
        value.verify_root()?;
        Ok(value)
    }
    /// Activates exactly one production mutation capability. Ordinary
    /// construction remains read-only; only the sealed repository-fit
    /// authority can obtain and move the grant consumed here.
    pub(in crate::repository_fit) fn open_with_mutation_grant(
        root: impl AsRef<Path>,
        unix_modes: BTreeMap<String, u32>,
        _grant: LocalMutationGrant,
    ) -> Result<Self, FitError> {
        let mut value = Self::open(root, unix_modes)?;
        value.mutation_lease = true;
        Ok(value)
    }
    #[cfg(test)]
    pub(crate) fn open_for_test(
        root: impl AsRef<Path>,
        unix_modes: BTreeMap<String, u32>,
    ) -> Result<Self, FitError> {
        let mut value = Self::open(root, unix_modes)?;
        value.mutation_lease = true;
        Ok(value)
    }
    #[cfg(test)]
    pub(crate) fn fail_on_calls(&mut self, calls: impl IntoIterator<Item = usize>) {
        self.fail_calls = calls.into_iter().collect();
    }
    #[cfg(test)]
    pub(crate) fn test_device_guard(root_device: u64, object_device: u64) -> Result<(), FitError> {
        require_same_device(root_device, object_device)
    }
    #[cfg(test)]
    pub(crate) fn pause_before_linearize(&mut self, reached: Arc<Barrier>, resume: Arc<Barrier>) {
        self.before_linearize = Some((reached, resume));
    }
    #[cfg(test)]
    pub(crate) fn pause_after_replace_swap(&mut self, reached: Arc<Barrier>, resume: Arc<Barrier>) {
        self.after_replace_swap = Some((reached, resume));
    }
    #[cfg(test)]
    pub(crate) fn pause_before_quarantine_move(
        &mut self,
        reached: Arc<Barrier>,
        resume: Arc<Barrier>,
    ) {
        self.before_quarantine_move = Some((reached, resume));
    }
    #[cfg(test)]
    pub(crate) fn pause_after_directory_publish(
        &mut self,
        reached: Arc<Barrier>,
        resume: Arc<Barrier>,
    ) {
        self.after_directory_publish = Some((reached, resume));
    }
    #[cfg(test)]
    pub(crate) fn pause_before_directory_quarantine_move(
        &mut self,
        reached: Arc<Barrier>,
        resume: Arc<Barrier>,
    ) {
        self.before_directory_quarantine_move = Some((reached, resume));
    }
    pub(crate) fn read_unix_mode(&mut self, path: &CanonicalPath) -> Result<Option<u32>, FitError> {
        let (parent, _) = match self.parent_anchor(path, false) {
            Ok(value) => value,
            Err(failure) if failure.id() == FitErrorId::Conflict => return Ok(None),
            Err(failure) => return Err(failure),
        };
        let name = path
            .components()
            .last()
            .expect("canonical path is nonempty");
        self.observe_leaf(&parent, name)
            .map(|observed| observed.map(|observed| observed.mode))
    }
    pub(crate) fn verify_root(&self) -> Result<(), FitError> {
        let path = fs::symlink_metadata(&self.path).map_err(|_| error(FitErrorId::StaleBinding))?;
        let opened = self
            .root
            .metadata()
            .map_err(|_| error(FitErrorId::StaleBinding))?;
        if path.file_type().is_symlink()
            || !path.is_dir()
            || object_identity(&path) != self.root_identity
            || object_identity(&opened) != self.root_identity
            || descriptor_path(&self.root)? != self.canonical
            || fs::canonicalize(&self.path).map_err(|_| error(FitErrorId::StaleBinding))?
                != self.canonical
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        Ok(())
    }
}
