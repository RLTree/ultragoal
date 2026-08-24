use crate::distribution::error::{DistributionError, DistributionErrorId, error};
use crate::distribution::package::tree_sha256;
use crate::distribution::reader::validate_relative_path;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

#[cfg(unix)]
use super::descriptor::{Directory, DirectoryIdentity};
#[cfg(unix)]
use super::hooks::{self, EffectPoint};
#[cfg(unix)]
use std::ffi::{CStr, CString};
#[cfg(unix)]
use std::fs::{File, Metadata};
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::path::Component;

/// One sealed, descriptor-anchored observation of a bounded directory tree.
///
/// This type intentionally exposes neither its retained descriptors nor any
/// mutation, recovery, rename, or removal operation. Personal-host reads must
/// never acquire the `ConfinedRoot` capability used by installation effects.
pub(crate) struct ReadOnlyTreeObservation {
    tree_sha256: String,
    observation_sha256: String,
    maximum_entries: usize,
    maximum_bytes: usize,
    #[cfg(unix)]
    chain: DirectoryChain,
    #[cfg(unix)]
    tree: Directory,
}

#[cfg(unix)]
#[derive(Serialize)]
struct ReadOnlyTreeDigestBinding<'a> {
    schema_version: &'static str,
    regular_file_tree_sha256: &'a str,
    directories: &'a [super::walk::DirectoryObservation],
}

impl std::fmt::Debug for ReadOnlyTreeObservation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReadOnlyTreeObservation")
            .field("tree_sha256", &self.tree_sha256)
            .field("observation_sha256", &self.observation_sha256)
            .field("maximum_entries", &self.maximum_entries)
            .field("maximum_bytes", &self.maximum_bytes)
            .finish_non_exhaustive()
    }
}

impl ReadOnlyTreeObservation {
    #[cfg(unix)]
    pub(crate) fn capture(
        canonical_home: &Path,
        relative: &str,
        maximum_entries: usize,
        maximum_bytes: usize,
    ) -> Result<Self, DistributionError> {
        validate_relative_path(relative)?;
        if maximum_entries == 0 || maximum_bytes == 0 || !canonical_home.is_absolute() {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let home_metadata = std::fs::symlink_metadata(canonical_home)
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        let observed_home = canonical_home
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?;
        if observed_home != canonical_home
            || home_metadata.file_type().is_symlink()
            || !home_metadata.is_dir()
        {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let target = canonical_home.join(relative);
        if target.strip_prefix(canonical_home).ok() != Some(Path::new(relative)) {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let chain = DirectoryChain::open(&target, canonical_home)?;
        if target
            .canonicalize()
            .map_err(|_| error(DistributionErrorId::ObjectUnavailable))?
            != target
        {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        hooks::before(EffectPoint::OpenDirectory, "read-only-tree-before-inspect");
        let tree = Directory::open_path(&target)?;
        if tree.identity() != chain.target_identity() {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        let mut observation = Self {
            tree_sha256: String::new(),
            observation_sha256: String::new(),
            maximum_entries,
            maximum_bytes,
            chain,
            tree,
        };
        let (tree_sha256, observation_sha256) = observation.observe_tree()?;
        observation.tree_sha256 = tree_sha256;
        observation.observation_sha256 = observation_sha256;
        Ok(observation)
    }

    #[cfg(not(unix))]
    pub(crate) fn capture(
        _canonical_home: &Path,
        _relative: &str,
        _maximum_entries: usize,
        _maximum_bytes: usize,
    ) -> Result<Self, DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    pub(crate) fn tree_sha256(&self) -> &str {
        &self.tree_sha256
    }

    pub(crate) fn observation_sha256(&self) -> &str {
        &self.observation_sha256
    }

    #[cfg(unix)]
    pub(crate) fn revalidate(&self) -> Result<(), DistributionError> {
        let (tree_sha256, observation_sha256) = self.observe_tree()?;
        if tree_sha256 != self.tree_sha256 || observation_sha256 != self.observation_sha256 {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub(crate) fn revalidate(&self) -> Result<(), DistributionError> {
        Err(error(DistributionErrorId::CapabilityMismatch))
    }

    #[cfg(unix)]
    fn observe_tree(&self) -> Result<(String, String), DistributionError> {
        self.chain.revalidate()?;
        self.tree.verify_descriptor()?;
        if self.tree.identity() != self.chain.target_identity() {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        let inspection = super::walk::inspect_with_directories(
            &self.tree,
            self.maximum_entries,
            self.maximum_bytes,
        )?;
        hooks::before(
            EffectPoint::OpenDirectory,
            "read-only-tree-before-final-revalidation",
        );
        self.tree.verify_descriptor()?;
        self.chain.revalidate()?;
        if self.tree.identity() != self.chain.target_identity() {
            return Err(error(DistributionErrorId::ObjectChanged));
        }
        let regular_file_tree_sha256 = tree_sha256(&inspection.files)?;
        let binding = ReadOnlyTreeDigestBinding {
            schema_version: "HarnessReadOnlyTreeObservation-v1",
            regular_file_tree_sha256: &regular_file_tree_sha256,
            directories: &inspection.directories,
        };
        let bytes =
            serde_json::to_vec(&binding).map_err(|_| error(DistributionErrorId::InvalidJson))?;
        Ok((
            regular_file_tree_sha256,
            format!("sha256:{:x}", Sha256::digest(bytes)),
        ))
    }
}

#[cfg(unix)]
struct DirectoryChain {
    rows: Vec<BoundDirectory>,
}

#[cfg(unix)]
struct BoundDirectory {
    directory: File,
    identity: DirectoryAttachmentIdentity,
    name_from_parent: Option<CString>,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DirectoryAttachmentIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    group: u32,
}

#[cfg(unix)]
impl DirectoryChain {
    fn open(target: &Path, canonical_home: &Path) -> Result<Self, DistributionError> {
        if !target.is_absolute() || !target.starts_with(canonical_home) {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let home_depth = normal_components(canonical_home)?;
        let root = open_root()?;
        let root_identity = directory_identity(&root.metadata().map_err(unavailable)?)?;
        let mut rows = vec![BoundDirectory {
            directory: root,
            identity: root_identity,
            name_from_parent: None,
        }];
        let mut depth = 0usize;
        let mut home_device = None;
        for component in target.components() {
            let Component::Normal(component) = component else {
                if matches!(component, Component::RootDir) {
                    continue;
                }
                return Err(error(DistributionErrorId::InvalidPath));
            };
            depth += 1;
            let name = CString::new(component.as_bytes())
                .map_err(|_| error(DistributionErrorId::InvalidPath))?;
            let parent = rows
                .last()
                .ok_or_else(|| error(DistributionErrorId::ObjectUnavailable))?;
            let directory = open_child(&parent.directory, &name)?;
            let identity = directory_identity(&directory.metadata().map_err(unavailable)?)?;
            let named = stat_child(&parent.directory, &name)?;
            if named != identity {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            if depth == home_depth {
                home_device = Some(identity.device);
            } else if depth > home_depth && home_device != Some(identity.device) {
                return Err(error(DistributionErrorId::UnsafeObject));
            }
            rows.push(BoundDirectory {
                directory,
                identity,
                name_from_parent: Some(name),
            });
        }
        if home_device.is_none() || rows.len() != normal_components(target)? + 1 {
            return Err(error(DistributionErrorId::InvalidPath));
        }
        let chain = Self { rows };
        chain.revalidate()?;
        Ok(chain)
    }

    fn target_identity(&self) -> DirectoryIdentity {
        let identity = self
            .rows
            .last()
            .expect("read-only directory chain retains its target")
            .identity;
        DirectoryIdentity {
            device: identity.device,
            inode: identity.inode,
        }
    }

    fn revalidate(&self) -> Result<(), DistributionError> {
        for (index, row) in self.rows.iter().enumerate() {
            if directory_identity(&row.directory.metadata().map_err(changed)?)? != row.identity {
                return Err(error(DistributionErrorId::ObjectChanged));
            }
            if index > 0 {
                let parent = &self.rows[index - 1];
                let name = row
                    .name_from_parent
                    .as_deref()
                    .expect("non-root directory retains its parent name");
                if stat_child(&parent.directory, name)? != row.identity {
                    return Err(error(DistributionErrorId::ObjectChanged));
                }
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
fn normal_components(path: &Path) -> Result<usize, DistributionError> {
    let mut count = 0usize;
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(_) => count += 1,
            _ => return Err(error(DistributionErrorId::InvalidPath)),
        }
    }
    Ok(count)
}

#[cfg(unix)]
fn open_root() -> Result<File, DistributionError> {
    let root = c"/";
    // SAFETY: the fixed root path is NUL-terminated and the flags are constants.
    let descriptor = unsafe {
        libc::open(
            root.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    owned_directory(descriptor)
}

#[cfg(unix)]
fn open_child(parent: &File, name: &CStr) -> Result<File, DistributionError> {
    // SAFETY: `parent` is retained and `name` is NUL-terminated for this call.
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    owned_directory(descriptor)
}

#[cfg(unix)]
fn owned_directory(descriptor: libc::c_int) -> Result<File, DistributionError> {
    if descriptor < 0 {
        return Err(error(DistributionErrorId::ObjectUnavailable));
    }
    // SAFETY: a successful open returns one unique owned descriptor.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn stat_child(
    parent: &File,
    name: &CStr,
) -> Result<DirectoryAttachmentIdentity, DistributionError> {
    let mut value = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: the parent descriptor and name are live and `value` is writable.
    let result = unsafe {
        libc::fstatat(
            parent.as_raw_fd(),
            name.as_ptr(),
            value.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        return Err(error(DistributionErrorId::ObjectChanged));
    }
    // SAFETY: successful fstatat initialized `value`.
    let value = unsafe { value.assume_init() };
    directory_identity_from_stat(&value)
}

#[cfg(unix)]
fn directory_identity(
    metadata: &Metadata,
) -> Result<DirectoryAttachmentIdentity, DistributionError> {
    if !metadata.is_dir() {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(DirectoryAttachmentIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        owner: metadata.uid(),
        group: metadata.gid(),
    })
}

#[cfg(unix)]
fn directory_identity_from_stat(
    value: &libc::stat,
) -> Result<DirectoryAttachmentIdentity, DistributionError> {
    if value.st_mode as libc::mode_t & libc::S_IFMT != libc::S_IFDIR {
        return Err(error(DistributionErrorId::UnsafeObject));
    }
    Ok(DirectoryAttachmentIdentity {
        device: value.st_dev as u64,
        inode: value.st_ino,
        mode: value.st_mode as u32,
        owner: value.st_uid,
        group: value.st_gid,
    })
}

#[cfg(unix)]
fn unavailable(_: std::io::Error) -> DistributionError {
    error(DistributionErrorId::ObjectUnavailable)
}

#[cfg(unix)]
fn changed(_: std::io::Error) -> DistributionError {
    error(DistributionErrorId::ObjectChanged)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::distribution::filesystem::{
        ConfinedRoot, assert_test_effect_hook_consumed, set_test_effect_hook_matching,
    };
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        home: PathBuf,
        target: PathBuf,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let requested = Path::new("/tmp").canonicalize().unwrap().join(format!(
                "hul-read-only-tree-{label}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&requested).unwrap();
            let root = requested.canonicalize().unwrap();
            let home = root.join("home");
            let target = home.join("market/source");
            fs::create_dir_all(&target).unwrap();
            Self { root, home, target }
        }

        fn capture(
            &self,
            maximum_entries: usize,
            maximum_bytes: usize,
        ) -> Result<ReadOnlyTreeObservation, DistributionError> {
            ReadOnlyTreeObservation::capture(
                &self.home,
                "market/source",
                maximum_entries,
                maximum_bytes,
            )
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn canonical_non_disposable_tree_is_observed_without_minting_confined_root_authority() {
        let fixture = Fixture::new("positive");
        fs::write(fixture.target.join("file.txt"), b"stable").unwrap();

        let observation = fixture.capture(16, 1024).unwrap();

        assert!(observation.tree_sha256().starts_with("sha256:"));
        observation.revalidate().unwrap();
        assert_eq!(
            ConfinedRoot::open(&fixture.target).unwrap_err().id(),
            DistributionErrorId::InvalidPath
        );
        assert_eq!(
            fs::read(fixture.target.join("file.txt")).unwrap(),
            b"stable"
        );
    }

    #[test]
    fn symlink_hardlink_and_fifo_descendants_fail_closed() {
        let linked = Fixture::new("symlink");
        let outside = linked.root.join("outside");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, linked.target.join("linked")).unwrap();
        assert!(linked.capture(16, 1024).is_err());
        assert_eq!(fs::read(&outside).unwrap(), b"outside");

        let hard = Fixture::new("hardlink");
        fs::write(hard.target.join("file"), b"linked").unwrap();
        fs::hard_link(hard.target.join("file"), hard.target.join("alias")).unwrap();
        assert_eq!(
            hard.capture(16, 1024).unwrap_err().id(),
            DistributionErrorId::UnsafeObject
        );

        let fifo = Fixture::new("fifo");
        let fifo_path = fifo.target.join("special");
        let encoded = CString::new(fifo_path.as_os_str().as_bytes()).unwrap();
        // SAFETY: `encoded` is a NUL-terminated path owned for this call.
        assert_eq!(unsafe { libc::mkfifo(encoded.as_ptr(), 0o600) }, 0);
        assert_eq!(
            fifo.capture(16, 1024).unwrap_err().id(),
            DistributionErrorId::UnsafeObject
        );
    }

    #[test]
    fn symlinked_ancestor_and_target_are_never_followed() {
        let ancestor = Fixture::new("ancestor-link");
        let real = ancestor.home.join("real");
        fs::create_dir_all(real.join("source")).unwrap();
        fs::write(real.join("source/file"), b"outside").unwrap();
        fs::remove_dir_all(ancestor.home.join("market")).unwrap();
        symlink(&real, ancestor.home.join("market")).unwrap();
        assert!(ancestor.capture(16, 1024).is_err());
        assert_eq!(fs::read(real.join("source/file")).unwrap(), b"outside");

        let target = Fixture::new("target-link");
        let real = target.home.join("real-source");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("file"), b"outside").unwrap();
        fs::remove_dir(&target.target).unwrap();
        symlink(&real, &target.target).unwrap();
        assert!(target.capture(16, 1024).is_err());
        assert_eq!(fs::read(real.join("file")).unwrap(), b"outside");
    }

    #[test]
    fn aggregate_entry_byte_and_depth_limits_include_directories() {
        let entries = Fixture::new("entry-limit");
        for outer in 0..65 {
            let outer = entries.target.join(format!("outer-{outer}"));
            fs::create_dir(&outer).unwrap();
            for inner in 0..64 {
                fs::create_dir(outer.join(format!("inner-{inner}"))).unwrap();
            }
        }
        assert_eq!(
            entries.capture(1, 1024).unwrap_err().id(),
            DistributionErrorId::ObjectTooLarge
        );

        let bytes = Fixture::new("byte-limit");
        fs::write(bytes.target.join("large"), b"two").unwrap();
        assert_eq!(
            bytes.capture(16, 1).unwrap_err().id(),
            DistributionErrorId::ObjectTooLarge
        );

        let depth = Fixture::new("depth-limit");
        let mut current = depth.target.clone();
        for _ in 0..66 {
            current.push("d");
        }
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("leaf"), b"leaf").unwrap();
        assert_eq!(
            depth.capture(128, 1024).unwrap_err().id(),
            DistributionErrorId::InvalidPath
        );
    }

    #[test]
    fn target_and_ancestor_substitution_fail_before_tree_admission() {
        let target = Fixture::new("target-swap");
        fs::write(target.target.join("original"), b"original").unwrap();
        let displaced = target.home.join("market/source-original");
        let replacement = target.target.clone();
        let moved = target.target.clone();
        set_test_effect_hook_matching(
            EffectPoint::OpenDirectory,
            "read-only-tree-before-inspect",
            move |_| {
                fs::rename(&moved, &displaced).unwrap();
                fs::create_dir(&replacement).unwrap();
                fs::write(replacement.join("replacement"), b"replacement").unwrap();
            },
        );
        assert_eq!(
            target.capture(16, 1024).unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
        assert_test_effect_hook_consumed();
        assert_eq!(
            fs::read(target.target.join("replacement")).unwrap(),
            b"replacement"
        );

        let ancestor = Fixture::new("ancestor-swap");
        fs::write(ancestor.target.join("original"), b"original").unwrap();
        let market = ancestor.home.join("market");
        let displaced = ancestor.home.join("market-original");
        let replacement = market.clone();
        set_test_effect_hook_matching(
            EffectPoint::OpenDirectory,
            "read-only-tree-before-inspect",
            move |_| {
                fs::rename(&market, &displaced).unwrap();
                fs::create_dir(&replacement).unwrap();
                fs::create_dir(replacement.join("source")).unwrap();
                fs::write(replacement.join("source/replacement"), b"replacement").unwrap();
            },
        );
        assert_eq!(
            ancestor.capture(16, 1024).unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
        assert_test_effect_hook_consumed();
    }

    #[test]
    fn final_ancestor_metadata_drift_and_later_content_drift_are_rejected() {
        let metadata = Fixture::new("metadata-drift");
        fs::write(metadata.target.join("file"), b"stable").unwrap();
        let changed = metadata.home.join("market");
        fs::set_permissions(&changed, fs::Permissions::from_mode(0o755)).unwrap();
        set_test_effect_hook_matching(
            EffectPoint::OpenDirectory,
            "read-only-tree-before-final-revalidation",
            move |_| {
                fs::set_permissions(&changed, fs::Permissions::from_mode(0o700)).unwrap();
            },
        );
        assert_eq!(
            metadata.capture(16, 1024).unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
        assert_test_effect_hook_consumed();

        let content = Fixture::new("content-drift");
        let file = content.target.join("file");
        fs::write(&file, b"before").unwrap();
        let observation = content.capture(16, 1024).unwrap();
        fs::write(&file, b"after!").unwrap();
        assert_eq!(
            observation.revalidate().unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );

        let topology = Fixture::new("directory-topology-drift");
        fs::write(topology.target.join("file"), b"stable").unwrap();
        let observation = topology.capture(16, 1024).unwrap();
        let prior_tree_sha256 = observation.tree_sha256().to_owned();
        let prior_observation_sha256 = observation.observation_sha256().to_owned();
        fs::create_dir(topology.target.join("later-empty-directory")).unwrap();
        assert_eq!(
            observation.revalidate().unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
        let current = topology.capture(16, 1024).unwrap();
        assert_eq!(current.tree_sha256(), prior_tree_sha256);
        assert_ne!(current.observation_sha256(), prior_observation_sha256);

        let descendant_mode = Fixture::new("descendant-directory-mode-drift");
        let descendant = descendant_mode.target.join("nested");
        fs::create_dir(&descendant).unwrap();
        fs::set_permissions(&descendant, fs::Permissions::from_mode(0o755)).unwrap();
        let observation = descendant_mode.capture(16, 1024).unwrap();
        fs::set_permissions(&descendant, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            observation.revalidate().unwrap_err().id(),
            DistributionErrorId::ObjectChanged
        );
    }
}
