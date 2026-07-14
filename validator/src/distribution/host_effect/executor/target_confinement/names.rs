impl ConfinedHostEffectTarget {
    fn names(&self) -> Result<Vec<String>, HostEffectExecutorFailure> {
        self.revalidate_anchor()?;
        let duplicate =
            unsafe { libc::fcntl(self.anchor.directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
        if duplicate < 0 {
            return Err(io_failure());
        }
        let directory = unsafe { libc::fdopendir(duplicate) };
        if directory.is_null() {
            unsafe { libc::close(duplicate) };
            return Err(io_failure());
        }
        let mut names = Vec::new();
        loop {
            clear_errno();
            let entry = unsafe { libc::readdir(directory) };
            if entry.is_null() {
                let read_error = last_errno();
                if read_error.is_some_and(|value| value != 0) {
                    unsafe { libc::closedir(directory) };
                    return Err(io_failure());
                }
                break;
            }
            let name = match unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_str() {
                Ok(name) => name,
                Err(_) => {
                    unsafe { libc::closedir(directory) };
                    return Err(unsafe_object());
                }
            };
            if !matches!(name, "." | "..") {
                if !valid_component(name) {
                    unsafe { libc::closedir(directory) };
                    return Err(unsafe_object());
                }
                names.push(name.to_owned());
            }
        }
        if unsafe { libc::closedir(directory) } != 0 {
            return Err(io_failure());
        }
        names.sort();
        names.dedup();
        self.revalidate_anchor()?;
        Ok(names)
    }
}

pub(crate) struct ConfinedHostEffectTargetObserver {
    target: ConfinedHostEffectTarget,
}

impl HostTargetObserver for ConfinedHostEffectTargetObserver {
    fn acquire(
        &mut self,
        expected: &ObservedTargetIdentity,
    ) -> Result<Box<dyn HostTargetLease>, SupportedHostLifecycleError> {
        let current = self
            .target
            .current_target_identity()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        if &current != expected || expected != self.target.expected_target() {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::TargetSubstitution,
            ));
        }
        Ok(Box::new(ConfinedHostEffectTargetLease {
            target: self.target.clone(),
            identity: current,
        }))
    }
}

struct ConfinedHostEffectTargetLease {
    target: ConfinedHostEffectTarget,
    identity: ObservedTargetIdentity,
}

impl HostTargetLease for ConfinedHostEffectTargetLease {
    fn identity(&self) -> &ObservedTargetIdentity {
        &self.identity
    }

    fn revalidate(&mut self) -> Result<ObservedTargetIdentity, SupportedHostLifecycleError> {
        let current = self
            .target
            .current_target_identity()
            .map_err(|_| lifecycle_error(SupportedHostLifecycleErrorId::TargetSubstitution))?;
        if current != self.identity {
            return Err(lifecycle_error(
                SupportedHostLifecycleErrorId::TargetSubstitution,
            ));
        }
        Ok(current)
    }
}

pub(super) struct PreparedPublication {
    target_name: String,
    temporary_name: String,
    bytes: Vec<u8>,
    expectation: PublicationExpectation,
}

pub(super) struct CommittedPublication {
    pub(super) target_name: String,
    pub(super) temporary_name: String,
    pub(super) expectation: PublicationExpectation,
    pub(super) observation: PublicationInventoryObservation,
}

pub(super) struct PublicationFailure {
    pub(super) id: HostEffectExecutorErrorId,
    target_name: String,
    temporary_name: String,
    expectation: PublicationExpectation,
    pub(super) observation: Option<PublicationInventoryObservation>,
}

struct ObservedObject {
    kind: PublicationObjectKind,
    observation: PublicationObjectObservation,
}

struct FileSnapshot {
    identity: ObjectIdentity,
    bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl ObjectIdentity {
    fn capture(metadata: &Metadata) -> Result<Self, HostEffectExecutorFailure> {
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    fn regular(self) -> bool {
        self.mode & u32::from(libc::S_IFMT) == u32::from(libc::S_IFREG)
    }

    fn kind(self) -> PublicationObjectKind {
        match self.mode & u32::from(libc::S_IFMT) {
            value if value == u32::from(libc::S_IFREG) => PublicationObjectKind::Regular,
            value if value == u32::from(libc::S_IFLNK) => PublicationObjectKind::Symlink,
            value if value == u32::from(libc::S_IFDIR) => PublicationObjectKind::Directory,
            value if value == u32::from(libc::S_IFIFO) => PublicationObjectKind::Fifo,
            value if value == u32::from(libc::S_IFSOCK) => PublicationObjectKind::Socket,
            value if value == u32::from(libc::S_IFCHR) || value == u32::from(libc::S_IFBLK) => {
                PublicationObjectKind::Device
            }
            _ => PublicationObjectKind::Unknown,
        }
    }

    fn same_after_rename(self, prior: Self) -> bool {
        self.device == prior.device
            && self.inode == prior.inode
            && self.mode == prior.mode
            && self.uid == prior.uid
            && self.gid == prior.gid
            && self.links == prior.links
            && self.length == prior.length
            && self.modified_seconds == prior.modified_seconds
            && self.modified_nanoseconds == prior.modified_nanoseconds
    }
}
