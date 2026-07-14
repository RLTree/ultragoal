#[cfg(all(test, unix))]
thread_local! {
    static BEFORE_CAPTURE_HOOK: std::cell::RefCell<Option<Box<dyn FnMut(&OsStr)>>> =
        std::cell::RefCell::new(None);
}

#[cfg(all(test, unix))]
pub(crate) fn set_before_capture_hook(hook: Option<Box<dyn FnMut(&OsStr)>>) {
    BEFORE_CAPTURE_HOOK.with(|slot| *slot.borrow_mut() = hook);
}

#[cfg(all(test, unix))]
fn run_before_capture_hook(name: &CStr) {
    let name = OsStr::from_bytes(name.to_bytes());
    BEFORE_CAPTURE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().as_mut() {
            hook(name);
        }
    });
}

#[cfg(all(not(test), unix))]
fn run_before_capture_hook(_name: &CStr) {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceBinding {
    pub kind: ResourceKind,
    pub key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LeaseDisposition {
    Active,
    Cleaned,
    RecoveryRequired,
}

#[derive(Debug)]
pub struct IsolationLease {
    id: String,
    fixture_id: String,
    root: PathBuf,
    #[cfg(unix)]
    pinned_root: PinnedLeaseRoot,
    bindings: Vec<ResourceBinding>,
    // This reservation deliberately remains open for the entire lease.  A
    // namespace string is not a port reservation: another fixture could bind
    // it between scheduling and launch.
    reserved_port: Option<TcpListener>,
    disposition: LeaseDisposition,
}

impl IsolationLease {
    pub(crate) fn acquire(
        root: &Path,
        spec: &FixtureSpec,
        ordinal: u64,
    ) -> Result<Self, FixtureScheduleError> {
        let id = format!(
            "{}-{}",
            spec.id,
            stable_digest(&format!("{}:{ordinal}", spec.metadata_digest))
        );
        let lease_root = root.join(&id);
        fs::create_dir_all(root)?;
        fs::create_dir(&lease_root).map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                FixtureScheduleError::Collision(id.clone())
            } else {
                FixtureScheduleError::Io(source)
            }
        })?;
        fs::write(
            lease_root.join(".fixture-lease"),
            format!("fixture={}\n", spec.id),
        )?;
        let mut reserved_port = None;
        let mut bindings = Vec::with_capacity(spec.resources.len());
        for kind in &spec.resources {
            let resource_root = lease_root.join(kind.label());
            let key = if *kind == ResourceKind::Port {
                let listener = TcpListener::bind(("127.0.0.1", 0))?;
                let address = listener.local_addr()?.to_string();
                fs::create_dir(&resource_root)?;
                fs::write(resource_root.join("reservation"), &address)?;
                reserved_port = Some(listener);
                address
            } else {
                fs::create_dir(&resource_root)?;
                resource_root
                    .to_str()
                    .ok_or_else(|| {
                        FixtureScheduleError::InvalidMetadata("non UTF-8 lease root".to_owned())
                    })?
                    .to_owned()
            };
            bindings.push(ResourceBinding {
                kind: kind.clone(),
                key,
            });
        }
        #[cfg(unix)]
        let pinned_root = PinnedLeaseRoot::pin(root, &lease_root)?;
        Ok(Self {
            id,
            fixture_id: spec.id.clone(),
            root: lease_root,
            #[cfg(unix)]
            pinned_root,
            bindings,
            reserved_port,
            disposition: LeaseDisposition::Active,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn fixture_id(&self) -> &str {
        &self.fixture_id
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn bindings(&self) -> &[ResourceBinding] {
        &self.bindings
    }
    pub fn disposition(&self) -> &LeaseDisposition {
        &self.disposition
    }

    pub fn cleanup(&mut self) -> Result<(), FixtureScheduleError> {
        if self.disposition == LeaseDisposition::Cleaned {
            return Ok(());
        }
        // Cleanup is a state transition, not just an I/O attempt. Mark the
        // lease recoverable before crossing the destructive boundary so every
        // error path is observable as RecoveryRequired by the scheduler that
        // retains it. Successful identity-conditioned deletion is the only
        // transition from this state to Cleaned.
        self.disposition = LeaseDisposition::RecoveryRequired;
        // Dropping the listener before recursive removal makes port release a
        // causal part of successful cleanup, rather than an eventual Drop.
        self.reserved_port.take();
        #[cfg(unix)]
        self.pinned_root
            .remove()
            .map_err(|source| FixtureScheduleError::cleanup(&self.id, source))?;
        #[cfg(not(unix))]
        remove_regular_tree(&self.root)
            .map_err(|source| FixtureScheduleError::cleanup(&self.id, source))?;
        self.disposition = LeaseDisposition::Cleaned;
        Ok(())
    }

    pub fn recover(&mut self) -> Result<(), FixtureScheduleError> {
        self.cleanup()
    }
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryKind {
    Directory,
    RegularFile,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EntryIdentity {
    device: u64,
    inode: u64,
    kind: EntryKind,
}

#[cfg(unix)]
#[derive(Debug)]
struct PinnedLeaseRoot {
    parent: fs::File,
    root: OwnedFd,
    name: CString,
    identity: EntryIdentity,
    initial_entries: BTreeMap<PathBuf, EntryIdentity>,
}
