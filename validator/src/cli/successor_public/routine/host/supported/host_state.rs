use super::*;

impl HostState {
    pub(crate) fn open(home: &Path) -> Result<Self, HostFailure> {
        if !home.is_absolute()
            || fs::canonicalize(home).map_err(|_| HostFailure::Unavailable)? != home
        {
            return Err(HostFailure::Invalid);
        }
        let home_directory = AnchoredDirectory::open_absolute(home)?;
        let mut current = home_directory.open_child(STATE_COMPONENTS[0])?;
        for component in &STATE_COMPONENTS[1..] {
            current = current.open_child(component)?;
        }
        let authority = current.open_child(AUTHORITY_DIRECTORY)?;
        let adapter = current.open_child(ADAPTER_DIRECTORY)?;
        let lock = adapter.open_regular(LOCK_NAME, libc::O_RDWR, 0o600)?;
        let lock_identity = identity(&lock.metadata().map_err(|_| HostFailure::Invalid)?);
        let lock = ProcessLock::acquire(lock)?.0;
        let mut marker_file = lock.try_clone().map_err(|_| HostFailure::Invalid)?;
        marker_file
            .seek(SeekFrom::Start(0))
            .map_err(|_| HostFailure::Invalid)?;
        let mut marker = Vec::new();
        marker_file
            .take((LOCK_MARKER.len() + 1) as u64)
            .read_to_end(&mut marker)
            .map_err(|_| HostFailure::Invalid)?;
        if marker != LOCK_MARKER {
            return Err(HostFailure::Invalid);
        }
        let state = Self {
            home: home_directory,
            state: current,
            authority,
            adapter,
            lock,
            lock_identity,
        };
        state.verify()?;
        Ok(state)
    }

    pub(crate) fn verify(&self) -> Result<(), HostFailure> {
        self.home.verify()?;
        self.state.verify()?;
        self.authority.verify()?;
        self.adapter.verify()?;
        let metadata = self.lock.metadata().map_err(|_| HostFailure::Invalid)?;
        if identity(&metadata) != self.lock_identity
            || self.adapter.stat(LOCK_NAME)? != Some(self.lock_identity)
        {
            return Err(HostFailure::Invalid);
        }
        Ok(())
    }
}
