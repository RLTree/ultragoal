#[cfg(unix)]
impl PinnedLeaseRoot {
    fn pin(parent_path: &Path, root_path: &Path) -> io::Result<Self> {
        let name = component_c_string(
            root_path
                .file_name()
                .ok_or_else(|| io::Error::other("lease root has no final component"))?,
        )?;
        let parent = open_directory(parent_path)
            .map_err(|error| io::Error::other(format!("pin lease parent: {error}")))?;
        let root = open_directory_at(parent.as_raw_fd(), &name)
            .map_err(|error| io::Error::other(format!("pin lease root: {error}")))?;
        let identity = identity_for_fd(root.as_raw_fd())
            .map_err(|error| io::Error::other(format!("stat pinned lease root: {error}")))?;
        if identity.kind != EntryKind::Directory {
            return Err(io::Error::other("lease root is not a directory"));
        }
        require_entry_identity(parent.as_raw_fd(), &name, identity)
            .map_err(|error| io::Error::other(format!("bind pinned lease root: {error}")))?;

        let mut initial_entries = BTreeMap::new();
        snapshot_initial_tree(root.as_raw_fd(), Path::new(""), &mut initial_entries)
            .map_err(|error| io::Error::other(format!("snapshot lease root: {error}")))?;
        Ok(Self {
            parent,
            root,
            name,
            identity,
            initial_entries,
        })
    }

    fn remove(&self) -> io::Result<()> {
        let guard = RootGuard {
            parent_fd: self.parent.as_raw_fd(),
            name: &self.name,
            identity: self.identity,
        };
        guard.validate()?;
        remove_directory_contents(
            self.root.as_raw_fd(),
            Path::new(""),
            &self.initial_entries,
            guard,
        )?;
        guard.validate()?;
        capture_and_remove(
            self.parent.as_raw_fd(),
            &self.name,
            self.root.as_raw_fd(),
            self.identity,
            guard,
        )?;
        if entry_identity(self.parent.as_raw_fd(), &self.name)?.is_some() {
            return Err(io::Error::other("lease root was replaced during cleanup"));
        }
        Ok(())
    }

    fn snapshot_provisioned_entries(&mut self) -> io::Result<()> {
        self.initial_entries.clear();
        snapshot_initial_tree(self.root.as_raw_fd(), Path::new(""), &mut self.initial_entries)
    }
}

#[cfg(unix)]
#[derive(Clone, Copy)]
struct RootGuard<'a> {
    parent_fd: RawFd,
    name: &'a CStr,
    identity: EntryIdentity,
}

#[cfg(unix)]
impl RootGuard<'_> {
    fn validate(self) -> io::Result<()> {
        require_entry_identity(self.parent_fd, self.name, self.identity)
            .map_err(|_| io::Error::other("lease root identity changed"))
    }
}

#[cfg(unix)]
fn snapshot_initial_tree(
    directory_fd: RawFd,
    relative: &Path,
    output: &mut BTreeMap<PathBuf, EntryIdentity>,
) -> io::Result<()> {
    for name in read_directory_names(directory_fd)? {
        let name_c = component_c_string(&name)?;
        let identity = entry_identity(directory_fd, &name_c)?
            .ok_or_else(|| io::Error::other("lease entry vanished during acquisition"))?;
        let child_relative = relative.join(&name);
        output.insert(child_relative.clone(), identity);
        if identity.kind == EntryKind::Directory {
            let child = open_directory_at(directory_fd, &name_c)?;
            require_entry_identity(directory_fd, &name_c, identity)?;
            snapshot_initial_tree(child.as_raw_fd(), &child_relative, output)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn remove_directory_contents(
    directory_fd: RawFd,
    relative: &Path,
    initial_entries: &BTreeMap<PathBuf, EntryIdentity>,
    guard: RootGuard<'_>,
) -> io::Result<()> {
    guard.validate()?;
    for name in read_directory_names(directory_fd)? {
        guard.validate()?;
        let name_c = component_c_string(&name)?;
        let child_relative = relative.join(&name);
        let observed = entry_identity(directory_fd, &name_c)?
            .ok_or_else(|| io::Error::other("lease entry vanished during cleanup"))?;

        if let Some(expected) = initial_entries.get(&child_relative) {
            if *expected != observed {
                return Err(io::Error::other("initial lease entry identity changed"));
            }
        } else if relative.as_os_str().is_empty() {
            // The confinement contract gives fixtures resource-specific
            // namespaces. A new top-level entry is therefore never scheduler
            // output and must not be recursively erased as though it were.
            return Err(io::Error::other("unexpected lease-root entry"));
        }

        let pinned = open_entry_at(directory_fd, &name_c, observed.kind)?;
        if identity_for_fd(pinned.as_raw_fd())? != observed {
            return Err(io::Error::other("lease child changed while being pinned"));
        }
        require_entry_identity(directory_fd, &name_c, observed)?;

        if observed.kind == EntryKind::Directory {
            remove_directory_contents(pinned.as_raw_fd(), &child_relative, initial_entries, guard)?;
        }
        guard.validate()?;
        require_entry_identity(directory_fd, &name_c, observed)?;
        capture_and_remove(directory_fd, &name_c, pinned.as_raw_fd(), observed, guard)?;
        if entry_identity(directory_fd, &name_c)?.is_some() {
            return Err(io::Error::other("lease child was replaced during cleanup"));
        }
    }
    Ok(())
}
