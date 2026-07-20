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
    custody: LeaseRootCustody,
    bindings: Vec<ResourceBinding>,
    // This reservation deliberately remains open for the entire lease.  A
    // namespace string is not a port reservation: another fixture could bind
    // it between scheduling and launch.
    reserved_port: Option<TcpListener>,
    disposition: LeaseDisposition,
}

#[derive(Debug)]
pub(crate) enum LeaseAcquisitionFailure { Failed(FixtureScheduleError), RecoveryRequired { lease: IsolationLease, source: FixtureScheduleError } }

#[cfg(all(test, unix))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LeaseAcquisitionStage { Pin, Marker, ResourceDirectory, PortBind, PortAddress, ReservationWrite, BeforeInsertion }

#[cfg(all(test, unix))]
thread_local! { static LEASE_ACQUISITION_HOOK: std::cell::RefCell<Option<Box<dyn FnMut(LeaseAcquisitionStage, &Path)>>> = std::cell::RefCell::new(None); }

#[cfg(all(test, unix))]
pub(crate) fn set_lease_acquisition_hook(hook: Option<Box<dyn FnMut(LeaseAcquisitionStage, &Path)>>) { LEASE_ACQUISITION_HOOK.with(|slot| *slot.borrow_mut() = hook); }

#[cfg(all(test, unix))]
pub(crate) fn run_lease_acquisition_hook(stage: LeaseAcquisitionStage, root: &Path) { LEASE_ACQUISITION_HOOK.with(|slot| if let Some(hook) = slot.borrow_mut().as_mut() { hook(stage, root) }); }

#[cfg(unix)] #[derive(Debug)]
enum LeaseRootCustody { Unpinned, Pinned(PinnedLeaseRoot) }

impl IsolationLease {
    pub(crate) fn acquire(
        root: &Path,
        spec: &FixtureSpec,
        ordinal: u64,
    ) -> Result<Self, LeaseAcquisitionFailure> {
        let id = format!(
            "{}-{}",
            spec.id,
            stable_digest(&format!("{}:{ordinal}", spec.metadata_digest))
        );
        let lease_root = root.join(&id);
        fs::create_dir_all(root).map_err(FixtureScheduleError::Io).map_err(LeaseAcquisitionFailure::Failed)?;
        let root_created = fs::create_dir(&lease_root).map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                FixtureScheduleError::Collision(id.clone())
            } else {
                FixtureScheduleError::Io(source)
            }
        });
        if let Err(FixtureScheduleError::Collision(_)) = root_created {
            #[cfg(unix)]
            {
                let lease = Self {
                    id,
                    fixture_id: spec.id.clone(),
                    root: lease_root,
                    custody: LeaseRootCustody::Unpinned,
                    bindings: Vec::new(),
                    reserved_port: None,
                    disposition: LeaseDisposition::RecoveryRequired,
                };
                let failure = FixtureScheduleError::cleanup(lease.id(), io::Error::new(io::ErrorKind::AlreadyExists, "deterministic provisional lease root exists"));
                return Err(LeaseAcquisitionFailure::RecoveryRequired { lease, source: failure });
            }
            #[cfg(not(unix))]
            return Err(LeaseAcquisitionFailure::Failed(source));
        }
        root_created.map_err(LeaseAcquisitionFailure::Failed)?;
        let mut lease = Self {
            id,
            fixture_id: spec.id.clone(),
            root: lease_root,
            #[cfg(unix)]
            custody: LeaseRootCustody::Unpinned,
            bindings: Vec::with_capacity(spec.resources.len()),
            reserved_port: None,
            disposition: LeaseDisposition::Active,
        };
        #[cfg(all(test, unix))]
        run_lease_acquisition_hook(LeaseAcquisitionStage::Pin, &lease.root);
        #[cfg(unix)]
        match PinnedLeaseRoot::pin(root, &lease.root) {
            Ok(pinned) => lease.custody = LeaseRootCustody::Pinned(pinned),
            Err(source) => {
                lease.disposition = LeaseDisposition::RecoveryRequired;
                let failure = FixtureScheduleError::cleanup(lease.id(), source);
                return Err(LeaseAcquisitionFailure::RecoveryRequired {
                    lease,
                    source: failure,
                });
            }
        }
        let provision = (|| -> Result<(), FixtureScheduleError> {
            #[cfg(all(test, unix))]
            run_lease_acquisition_hook(LeaseAcquisitionStage::Marker, &lease.root);
            fs::write(
                lease.root.join(".fixture-lease"),
                format!("fixture={}\n", spec.id),
            )?;
            for kind in &spec.resources {
                let resource_root = lease.root.join(kind.label());
                #[cfg(all(test, unix))]
                run_lease_acquisition_hook(LeaseAcquisitionStage::ResourceDirectory, &resource_root);
                let key = if *kind == ResourceKind::Port {
                    #[cfg(all(test, unix))]
                    run_lease_acquisition_hook(LeaseAcquisitionStage::PortBind, &resource_root);
                    let listener = TcpListener::bind(("127.0.0.1", 0))?;
                    #[cfg(all(test, unix))]
                    run_lease_acquisition_hook(LeaseAcquisitionStage::PortAddress, &resource_root);
                    let address = listener.local_addr()?.to_string();
                    fs::create_dir(&resource_root)?;
                    #[cfg(all(test, unix))]
                    run_lease_acquisition_hook(LeaseAcquisitionStage::ReservationWrite, &resource_root);
                    fs::write(resource_root.join("reservation"), &address)?;
                    lease.reserved_port = Some(listener);
                    address
                } else {
                    fs::create_dir(&resource_root)?;
                    resource_root.to_str().ok_or_else(|| {
                        FixtureScheduleError::InvalidMetadata("non UTF-8 lease root".to_owned())
                    })?.to_owned()
                };
                lease.bindings.push(ResourceBinding { kind: kind.clone(), key });
            }
            Ok(())
        })();
        match provision {
            Ok(()) => {
                #[cfg(unix)] let snapshot = match &mut lease.custody { LeaseRootCustody::Pinned(pinned) => pinned.snapshot_provisioned_entries(), LeaseRootCustody::Unpinned => Err(io::Error::other("pinned lease lost custody")) };
                #[cfg(unix)] if let Err(source) = snapshot { lease.disposition = LeaseDisposition::RecoveryRequired; let source = FixtureScheduleError::cleanup(lease.id(), source); return Err(LeaseAcquisitionFailure::RecoveryRequired { lease, source }); }
                Ok(lease)
            }
            Err(source) => match lease.cleanup() {
                Ok(()) => Err(LeaseAcquisitionFailure::Failed(source)),
                Err(_) => Err(LeaseAcquisitionFailure::RecoveryRequired { lease, source }),
            },
        }
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
        self.disposition = LeaseDisposition::RecoveryRequired;
        self.reserved_port.take();
        #[cfg(unix)]
        match &self.custody {
            LeaseRootCustody::Pinned(pinned_root) => pinned_root
                .remove()
                .map_err(|source| FixtureScheduleError::cleanup(&self.id, source))?,
            LeaseRootCustody::Unpinned => {
                return Err(FixtureScheduleError::cleanup(
                    &self.id,
                    io::Error::new(io::ErrorKind::Unsupported, "lease root is unpinned and retained for recovery"),
                ));
            }
        }
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
