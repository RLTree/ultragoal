use super::spec::stable_digest;
use super::{FixtureScheduleError, FixtureSpec, ResourceKind};
use std::collections::BTreeMap;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::ffi::{CStr, CString, OsStr, OsString};
#[cfg(unix)]
use std::io;
#[cfg(unix)]
use std::mem::MaybeUninit;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};

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

#[cfg(unix)]
fn capture_and_remove(
    parent_fd: RawFd,
    name: &CStr,
    expected_fd: RawFd,
    expected: EntryIdentity,
    guard: RootGuard<'_>,
) -> io::Result<()> {
    guard.validate()?;
    require_entry_identity(parent_fd, name, expected)?;
    run_before_capture_hook(name);

    #[cfg(target_os = "freebsd")]
    {
        // FreeBSD can condition the namespace removal on the already-open
        // descriptor in one syscall. Revalidate after the pre-capture hook,
        // then expose the final race window: funlinkat must reject any name
        // replacement without deleting either identity.
        guard.validate()?;
        require_entry_identity(parent_fd, name, expected)?;
        run_before_capture_hook(name);
        return identity_conditioned_unlink(parent_fd, name, expected_fd, expected.kind);
    }

    #[cfg(not(target_os = "freebsd"))]
    let quarantine = unused_quarantine_name(parent_fd)?;
    #[cfg(not(target_os = "freebsd"))]
    rename_noreplace(parent_fd, name, parent_fd, &quarantine)?;

    #[cfg(not(target_os = "freebsd"))]
    if require_entry_identity(parent_fd, &quarantine, expected).is_err() {
        restore_captured_entry(parent_fd, name, &quarantine, expected);
        return Err(io::Error::other("captured lease entry identity changed"));
    }

    // The destructive boundary must bind the name and the already-pinned
    // identity in one kernel operation. A final fstatat followed by unlinkat
    // is still replaceable between syscalls and can erase an unrelated entry.
    #[cfg(not(target_os = "freebsd"))]
    run_before_capture_hook(&quarantine);
    #[cfg(not(target_os = "freebsd"))]
    if let Err(error) =
        identity_conditioned_unlink(parent_fd, &quarantine, expected_fd, expected.kind)
    {
        // Restoration is non-destructive and is attempted only while the
        // captured name still denotes the expected identity. If an attacker
        // replaced it, retain both names for explicit recovery.
        restore_captured_entry(parent_fd, name, &quarantine, expected);
        return Err(error);
    }
    #[cfg(not(target_os = "freebsd"))]
    Ok(())
}

#[cfg(unix)]
fn restore_captured_entry(
    parent_fd: RawFd,
    original: &CStr,
    captured: &CStr,
    expected: EntryIdentity,
) {
    if matches!(entry_identity(parent_fd, original), Ok(None))
        && matches!(entry_identity(parent_fd, captured), Ok(Some(observed)) if observed == expected)
    {
        let _ = rename_noreplace(parent_fd, captured, parent_fd, original);
    }
}

#[cfg(target_os = "freebsd")]
fn identity_conditioned_unlink(
    parent_fd: RawFd,
    name: &CStr,
    expected_fd: RawFd,
    kind: EntryKind,
) -> io::Result<()> {
    unsafe extern "C" {
        fn funlinkat(
            directory_fd: libc::c_int,
            path: *const libc::c_char,
            file_fd: libc::c_int,
            flags: libc::c_int,
        ) -> libc::c_int;
    }

    let flags = if kind == EntryKind::Directory {
        libc::AT_REMOVEDIR
    } else {
        0
    };
    if unsafe { funlinkat(parent_fd, name.as_ptr(), expected_fd, flags) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(unix, not(target_os = "freebsd")))]
fn identity_conditioned_unlink(
    _parent_fd: RawFd,
    _name: &CStr,
    _expected_fd: RawFd,
    _kind: EntryKind,
) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "identity-conditioned filesystem deletion is unavailable on this platform; lease retained for recovery",
    ))
}

#[cfg(unix)]
fn unused_quarantine_name(parent_fd: RawFd) -> io::Result<CString> {
    for _ in 0..16 {
        let mut random = [0_u8; 16];
        fill_random(&mut random)?;
        let mut value = String::from(".hul-delete-");
        for byte in random {
            use std::fmt::Write as _;
            write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
        }
        let value = CString::new(value).expect("hex cleanup name contains no NUL");
        if entry_identity(parent_fd, &value)?.is_none() {
            return Ok(value);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate private cleanup name",
    ))
}

#[cfg(target_os = "macos")]
fn fill_random(output: &mut [u8]) -> io::Result<()> {
    unsafe { libc::arc4random_buf(output.as_mut_ptr().cast(), output.len()) };
    Ok(())
}

#[cfg(target_os = "linux")]
fn fill_random(output: &mut [u8]) -> io::Result<()> {
    let written = unsafe { libc::getrandom(output.as_mut_ptr().cast(), output.len(), 0) };
    if written == output.len() as isize {
        Ok(())
    } else if written < 0 {
        Err(io::Error::last_os_error())
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "short operating-system random read",
        ))
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn fill_random(_output: &mut [u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "secure cleanup nonce unavailable on this platform",
    ))
}

#[cfg(target_os = "macos")]
fn rename_noreplace(from_fd: RawFd, from: &CStr, to_fd: RawFd, to: &CStr) -> io::Result<()> {
    let result = unsafe {
        libc::renameatx_np(
            from_fd,
            from.as_ptr(),
            to_fd,
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
fn rename_noreplace(from_fd: RawFd, from: &CStr, to_fd: RawFd, to: &CStr) -> io::Result<()> {
    let result = unsafe {
        libc::syscall(
            libc::SYS_renameat2,
            from_fd,
            from.as_ptr(),
            to_fd,
            to.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn rename_noreplace(_from_fd: RawFd, _from: &CStr, _to_fd: RawFd, _to: &CStr) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace rename unavailable on this platform",
    ))
}

#[cfg(unix)]
fn open_directory(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
}

#[cfg(unix)]
fn open_directory_at(parent_fd: RawFd, name: &CStr) -> io::Result<OwnedFd> {
    open_at(
        parent_fd,
        name,
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
    )
}

#[cfg(unix)]
fn open_entry_at(parent_fd: RawFd, name: &CStr, kind: EntryKind) -> io::Result<OwnedFd> {
    let flags = match kind {
        EntryKind::Directory => {
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
        }
        EntryKind::RegularFile => {
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC
        }
    };
    open_at(parent_fd, name, flags)
}

#[cfg(unix)]
fn open_at(parent_fd: RawFd, name: &CStr, flags: libc::c_int) -> io::Result<OwnedFd> {
    let descriptor = unsafe { libc::openat(parent_fd, name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
    }
}

#[cfg(unix)]
fn identity_for_fd(fd: RawFd) -> io::Result<EntryIdentity> {
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    if unsafe { libc::fstat(fd, stat.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    identity_from_stat(unsafe { stat.assume_init() })
}

#[cfg(unix)]
fn entry_identity(parent_fd: RawFd, name: &CStr) -> io::Result<Option<EntryIdentity>> {
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            parent_fd,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        identity_from_stat(unsafe { stat.assume_init() }).map(Some)
    } else {
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

#[cfg(unix)]
fn identity_from_stat(stat: libc::stat) -> io::Result<EntryIdentity> {
    let file_type = stat.st_mode & libc::S_IFMT;
    let kind = if file_type == libc::S_IFDIR {
        EntryKind::Directory
    } else if file_type == libc::S_IFREG {
        EntryKind::RegularFile
    } else if file_type == libc::S_IFLNK {
        return Err(io::Error::other("lease contains a symbolic link"));
    } else {
        return Err(io::Error::other("lease contains a special file"));
    };
    Ok(EntryIdentity {
        device: stat.st_dev as u64,
        inode: stat.st_ino as u64,
        kind,
    })
}

#[cfg(unix)]
fn require_entry_identity(
    parent_fd: RawFd,
    name: &CStr,
    expected: EntryIdentity,
) -> io::Result<()> {
    match entry_identity(parent_fd, name)? {
        Some(observed) if observed == expected => Ok(()),
        _ => Err(io::Error::other("lease filesystem identity changed")),
    }
}

#[cfg(unix)]
fn read_directory_names(directory_fd: RawFd) -> io::Result<Vec<OsString>> {
    let descriptor = unsafe { libc::dup(directory_fd) };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    let directory = unsafe { libc::fdopendir(descriptor) };
    if directory.is_null() {
        let error = io::Error::last_os_error();
        unsafe { libc::close(descriptor) };
        return Err(error);
    }
    unsafe { libc::rewinddir(directory) };

    let mut names = Vec::new();
    loop {
        let entry = unsafe { libc::readdir(directory) };
        if entry.is_null() {
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name != b"." && name != b".." {
            names.push(OsString::from_vec(name.to_vec()));
        }
    }
    if unsafe { libc::closedir(directory) } != 0 {
        return Err(io::Error::last_os_error());
    }
    names.sort();
    Ok(names)
}

#[cfg(unix)]
fn component_c_string(value: &OsStr) -> io::Result<CString> {
    if value.as_bytes().contains(&b'/') {
        return Err(io::Error::other("lease entry is not one path component"));
    }
    CString::new(value.as_bytes())
        .map_err(|_| io::Error::other("lease entry contains an invalid NUL"))
}

#[cfg(not(unix))]
fn remove_regular_tree(_path: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "identity-conditioned filesystem deletion is unavailable on this platform; lease retained for recovery",
    ))
}

pub(crate) fn isolated_environment(lease: &IsolationLease) -> BTreeMap<String, String> {
    lease
        .bindings
        .iter()
        .map(|binding| {
            (
                format!("HUL_FIXTURE_{}", binding.kind.label().to_ascii_uppercase()),
                binding.key.clone(),
            )
        })
        .collect()
}
