use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[cfg(unix)]
use std::ffi::{CStr, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

use crate::routine_work::{RepoPath, RoutineError, RoutineErrorId};

use super::super::model::{RoutineReadAncestor, RoutineReadSource};
use super::model::OutputFileRecord;

const MAX_OUTPUT_FILES: usize = 100_000;
const MAX_READ_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ObjectIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    owner_user_id: u32,
    owner_group_id: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
}

#[cfg(unix)]
impl ObjectIdentity {
    fn from(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            owner_user_id: metadata.uid(),
            owner_group_id: metadata.gid(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
        }
    }

    fn same_anchor_object(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.mode == other.mode
    }
}

#[cfg(unix)]
struct ReadAncestorAnchor {
    file: File,
    identity: ObjectIdentity,
}

#[cfg(unix)]
struct ReadSourceAnchor {
    path: PathBuf,
    file: File,
    identity: ObjectIdentity,
    ancestors: Vec<ReadAncestorAnchor>,
    record: RoutineReadSource,
}

pub(super) struct ReadConfinement {
    #[cfg(unix)]
    sources: Vec<ReadSourceAnchor>,
}

impl ReadConfinement {
    pub(super) fn bind_records(
        root: &RootAnchor,
        sources: &[RepoPath],
    ) -> Result<Vec<RoutineReadSource>, RoutineError> {
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            root.validate()?;
            let mut consumed = 0_u64;
            let mut records = Vec::with_capacity(sources.len());
            for source in sources {
                let anchor = open_read_source(root, source)?;
                consumed = consumed
                    .checked_add(anchor.record.byte_length)
                    .filter(|total| *total <= MAX_READ_SOURCE_BYTES)
                    .ok_or_else(|| mediator_error("mediator-read-source-budget-exceeded"))?;
                records.push(anchor.record);
            }
            root.validate()?;
            Ok(records)
        }
    }

    pub(super) fn open_bound(
        root: &RootAnchor,
        expected: &[RoutineReadSource],
    ) -> Result<Self, RoutineError> {
        if expected.is_empty() {
            return Ok(Self {
                #[cfg(unix)]
                sources: Vec::new(),
            });
        }
        #[cfg(not(unix))]
        {
            let _ = root;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            root.validate()?;
            let mut consumed = 0_u64;
            let mut sources = Vec::with_capacity(expected.len());
            for expected in expected {
                let anchor = open_read_source(root, &expected.relative_path)?;
                consumed = consumed
                    .checked_add(anchor.record.byte_length)
                    .filter(|total| *total <= MAX_READ_SOURCE_BYTES)
                    .ok_or_else(|| mediator_error("mediator-read-source-budget-exceeded"))?;
                if &anchor.record != expected {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-binding-stale",
                        None,
                    ));
                }
                sources.push(anchor);
            }
            let confinement = Self { sources };
            confinement.validate(root)?;
            Ok(confinement)
        }
    }

    pub(super) fn absolute_sources(&self) -> Vec<&Path> {
        #[cfg(not(unix))]
        {
            Vec::new()
        }
        #[cfg(unix)]
        {
            self.sources
                .iter()
                .map(|source| source.path.as_path())
                .collect()
        }
    }

    pub(super) fn validate(&self, root: &RootAnchor) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = root;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            root.validate()?;
            for source in &self.sources {
                for (held, expected) in source.ancestors.iter().zip(source.record.ancestors.iter())
                {
                    let current = ObjectIdentity::from(&held.file.metadata().map_err(|_| {
                        mediator_error("mediator-read-source-ancestor-metadata-failed")
                    })?);
                    if current != held.identity || !ancestor_matches(expected, current) {
                        return Err(RoutineError::new(
                            RoutineErrorId::ConcurrentMutation,
                            "mediator-read-source-ancestor-changed",
                            None,
                        ));
                    }
                }
                let held = ObjectIdentity::from(
                    &source
                        .file
                        .metadata()
                        .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
                );
                let mut reader = source
                    .file
                    .try_clone()
                    .map_err(|_| mediator_error("mediator-read-source-open-failed"))?;
                reader
                    .seek(SeekFrom::Start(0))
                    .map_err(|_| mediator_error("mediator-read-source-read-failed"))?;
                let digest = digest_reader(&mut reader, source.record.byte_length)?;
                if held != source.identity || !source_matches(&source.record, held, &digest) {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-changed",
                        None,
                    ));
                }
                let current = open_read_source(root, &source.record.relative_path)?;
                if current.record != source.record {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-read-source-replaced",
                        None,
                    ));
                }
            }
            root.validate()?;
            Ok(())
        }
    }
}

#[cfg(unix)]
fn open_read_source(
    root: &RootAnchor,
    relative: &RepoPath,
) -> Result<ReadSourceAnchor, RoutineError> {
    let components = relative.as_str().split('/').collect::<Vec<_>>();
    let (file_name, directory_components) = components
        .split_last()
        .ok_or_else(|| mediator_error("mediator-read-source-path-invalid"))?;
    let mut directory = root
        .file
        .try_clone()
        .map_err(|_| mediator_error("mediator-read-source-ancestor-open-failed"))?;
    let mut ancestors = Vec::with_capacity(directory_components.len() + 1);
    let mut ancestor_records = Vec::with_capacity(directory_components.len() + 1);
    let root_identity = ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-ancestor-metadata-failed"))?,
    );
    ancestors.push(ReadAncestorAnchor {
        file: directory
            .try_clone()
            .map_err(|_| mediator_error("mediator-read-source-ancestor-open-failed"))?,
        identity: root_identity,
    });
    ancestor_records.push(read_ancestor_record(String::new(), root_identity));
    let mut accumulated = String::new();
    for component in directory_components {
        let encoded = CString::new(component.as_bytes())
            .map_err(|_| mediator_error("mediator-read-source-path-invalid"))?;
        let child = open_read_component(&directory, &encoded, true)?;
        let identity = ObjectIdentity::from(
            &child
                .metadata()
                .map_err(|_| mediator_error("mediator-read-source-ancestor-metadata-failed"))?,
        );
        if identity.device != root.identity.device
            || identity.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFDIR)
        {
            return Err(mediator_error("mediator-read-source-ancestor-unsafe"));
        }
        if !accumulated.is_empty() {
            accumulated.push('/');
        }
        accumulated.push_str(component);
        ancestors.push(ReadAncestorAnchor {
            file: child
                .try_clone()
                .map_err(|_| mediator_error("mediator-read-source-ancestor-open-failed"))?,
            identity,
        });
        ancestor_records.push(read_ancestor_record(accumulated.clone(), identity));
        directory = child;
    }
    let encoded = CString::new(file_name.as_bytes())
        .map_err(|_| mediator_error("mediator-read-source-path-invalid"))?;
    let mut file = open_read_component(&directory, &encoded, false)?;
    let before = ObjectIdentity::from(
        &file
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
    );
    if before.device != root.identity.device
        || before.mode & u32::from(libc::S_IFMT) != u32::from(libc::S_IFREG)
        || before.links != 1
        || before.length > MAX_READ_SOURCE_BYTES
    {
        return Err(mediator_error("mediator-read-source-object-unsafe"));
    }
    let sha256 = digest_reader(&mut file, before.length)?;
    run_test_read_source_capture_hook();
    let after = ObjectIdentity::from(
        &file
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
    );
    if before != after {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-read-source-mutated-during-capture",
            None,
        ));
    }
    let current = open_read_component(&directory, &encoded, false).map_err(|_| {
        RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-read-source-replaced-during-capture",
            None,
        )
    })?;
    let current = ObjectIdentity::from(
        &current
            .metadata()
            .map_err(|_| mediator_error("mediator-read-source-metadata-failed"))?,
    );
    if current != before {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-read-source-replaced-during-capture",
            None,
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|_| mediator_error("mediator-read-source-read-failed"))?;
    let record = RoutineReadSource {
        relative_path: relative.clone(),
        device: before.device,
        inode: before.inode,
        unix_mode: before.mode,
        owner_user_id: before.owner_user_id,
        owner_group_id: before.owner_group_id,
        link_count: before.links,
        byte_length: before.length,
        modified_seconds: before.modified_seconds,
        modified_nanos: before.modified_nanos,
        changed_seconds: before.changed_seconds,
        changed_nanos: before.changed_nanos,
        sha256,
        ancestors: ancestor_records,
    };
    Ok(ReadSourceAnchor {
        path: root.path.join(relative.as_str()),
        file,
        identity: before,
        ancestors,
        record,
    })
}

#[cfg(unix)]
fn open_read_component(
    directory: &File,
    name: &CStr,
    require_directory: bool,
) -> Result<File, RoutineError> {
    let directory_flag = if require_directory {
        libc::O_DIRECTORY
    } else {
        0
    };
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC | directory_flag,
        )
    };
    if descriptor < 0 {
        return Err(mediator_error("mediator-read-source-object-unsafe"));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn read_ancestor_record(
    relative_directory: String,
    identity: ObjectIdentity,
) -> RoutineReadAncestor {
    RoutineReadAncestor {
        relative_directory,
        device: identity.device,
        inode: identity.inode,
        unix_mode: identity.mode,
        owner_user_id: identity.owner_user_id,
        owner_group_id: identity.owner_group_id,
        link_count: identity.links,
        modified_seconds: identity.modified_seconds,
        modified_nanos: identity.modified_nanos,
        changed_seconds: identity.changed_seconds,
        changed_nanos: identity.changed_nanos,
    }
}

#[cfg(unix)]
fn ancestor_matches(record: &RoutineReadAncestor, identity: ObjectIdentity) -> bool {
    record.device == identity.device
        && record.inode == identity.inode
        && record.unix_mode == identity.mode
        && record.owner_user_id == identity.owner_user_id
        && record.owner_group_id == identity.owner_group_id
        && record.link_count == identity.links
        && record.modified_seconds == identity.modified_seconds
        && record.modified_nanos == identity.modified_nanos
        && record.changed_seconds == identity.changed_seconds
        && record.changed_nanos == identity.changed_nanos
}

#[cfg(unix)]
fn source_matches(record: &RoutineReadSource, identity: ObjectIdentity, digest: &str) -> bool {
    record.device == identity.device
        && record.inode == identity.inode
        && record.unix_mode == identity.mode
        && record.owner_user_id == identity.owner_user_id
        && record.owner_group_id == identity.owner_group_id
        && record.link_count == identity.links
        && record.byte_length == identity.length
        && record.modified_seconds == identity.modified_seconds
        && record.modified_nanos == identity.modified_nanos
        && record.changed_seconds == identity.changed_seconds
        && record.changed_nanos == identity.changed_nanos
        && record.sha256 == digest
}

pub(super) struct RootAnchor {
    path: PathBuf,
    file: File,
    #[cfg(unix)]
    identity: ObjectIdentity,
}

impl RootAnchor {
    pub(super) fn open(path: &Path) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let lexical = path.to_path_buf();
            let metadata = fs::symlink_metadata(&lexical)
                .map_err(|_| mediator_error("mediator-working-directory-unavailable"))?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(mediator_error(
                    "mediator-working-directory-identity-invalid",
                ));
            }
            let canonical = lexical
                .canonicalize()
                .map_err(|_| mediator_error("mediator-working-directory-unavailable"))?;
            if canonical != lexical {
                return Err(mediator_error("mediator-working-directory-not-canonical"));
            }
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&canonical)
                .map_err(|_| mediator_error("mediator-working-directory-open-failed"))?;
            let identity = ObjectIdentity::from(
                &file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-working-directory-metadata-failed"))?,
            );
            Ok(Self {
                path: canonical,
                file,
                identity,
            })
        }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(unix)]
    pub(super) fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    #[cfg(unix)]
    pub(super) fn device(&self) -> u64 {
        self.identity.device
    }

    pub(super) fn validate(&self) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let held = ObjectIdentity::from(
                &self
                    .file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-working-directory-metadata-failed"))?,
            );
            let current_meta = fs::symlink_metadata(&self.path)
                .map_err(|_| mediator_error("mediator-working-directory-replaced"))?;
            if current_meta.file_type().is_symlink() || !current_meta.is_dir() {
                return Err(mediator_error("mediator-working-directory-replaced"));
            }
            let current = ObjectIdentity::from(&current_meta);
            let canonical = self
                .path
                .canonicalize()
                .map_err(|_| mediator_error("mediator-working-directory-replaced"))?;
            if held != self.identity || current != self.identity || canonical != self.path {
                return Err(RoutineError::new(
                    RoutineErrorId::ConcurrentMutation,
                    "mediator-working-directory-replaced",
                    None,
                ));
            }
            Ok(())
        }
    }
}

pub(super) struct PinnedExecutable {
    path: PathBuf,
    file: File,
    #[cfg(unix)]
    identity: ObjectIdentity,
    sha256: String,
}

impl PinnedExecutable {
    pub(super) fn open_bound(
        path_hex: &str,
        expected_sha256: &str,
        expected_length: u64,
        expected_mode: Option<u32>,
    ) -> Result<Self, RoutineError> {
        let path = decode_path(path_hex)?;
        let executable = Self::open_unbound(&path)?;
        if executable.sha256 != expected_sha256
            || executable.identity_length() != expected_length
            || executable.identity_mode() != expected_mode
        {
            return Err(RoutineError::new(
                RoutineErrorId::ContextMismatch,
                "mediator-executable-binding-stale",
                None,
            ));
        }
        Ok(executable)
    }

    pub(super) fn open_unbound(path: &Path) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            if !path.is_absolute() {
                return Err(mediator_error("mediator-executable-path-invalid"));
            }
            validate_execution_path_immutability(path)?;
            let metadata = fs::symlink_metadata(path)
                .map_err(|_| mediator_error("mediator-executable-unavailable"))?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || metadata.nlink() != 1
                || metadata.permissions().mode() & 0o111 == 0
                || metadata.permissions().mode() & 0o022 != 0
            {
                return Err(mediator_error("mediator-executable-identity-unsafe"));
            }
            let canonical = path
                .canonicalize()
                .map_err(|_| mediator_error("mediator-executable-unavailable"))?;
            if &canonical != path {
                return Err(mediator_error("mediator-executable-not-canonical"));
            }
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(path)
                .map_err(|_| mediator_error("mediator-executable-open-failed"))?;
            let identity = ObjectIdentity::from(
                &file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
            );
            let sha256 = digest_reader(&mut file, u64::MAX)?;
            Ok(Self {
                path: path.to_path_buf(),
                file,
                identity,
                sha256,
            })
        }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn sha256(&self) -> &str {
        &self.sha256
    }

    #[cfg(unix)]
    fn identity_length(&self) -> u64 {
        self.identity.length
    }

    #[cfg(not(unix))]
    fn identity_length(&self) -> u64 {
        0
    }

    #[cfg(unix)]
    fn identity_mode(&self) -> Option<u32> {
        Some(self.identity.mode)
    }

    #[cfg(not(unix))]
    fn identity_mode(&self) -> Option<u32> {
        None
    }

    pub(super) fn validate(&self) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            let held = ObjectIdentity::from(
                &self
                    .file
                    .metadata()
                    .map_err(|_| mediator_error("mediator-executable-metadata-failed"))?,
            );
            let current_meta = fs::symlink_metadata(&self.path)
                .map_err(|_| mediator_error("mediator-executable-replaced"))?;
            if current_meta.file_type().is_symlink() || !current_meta.is_file() {
                return Err(mediator_error("mediator-executable-replaced"));
            }
            let current = ObjectIdentity::from(&current_meta);
            let mut reopened = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&self.path)
                .map_err(|_| mediator_error("mediator-executable-replaced"))?;
            let digest = digest_reader(&mut reopened, u64::MAX)?;
            validate_execution_path_immutability(&self.path)?;
            if held != self.identity || current != self.identity || digest != self.sha256 {
                return Err(RoutineError::new(
                    RoutineErrorId::ConcurrentMutation,
                    "mediator-executable-replaced",
                    None,
                ));
            }
            Ok(())
        }
    }
}

#[cfg(unix)]
fn validate_execution_path_immutability(path: &Path) -> Result<(), RoutineError> {
    let effective_user_id = unsafe { libc::geteuid() };
    let mut current = Some(path);
    while let Some(component) = current {
        let metadata = fs::symlink_metadata(component)
            .map_err(|_| mediator_error("mediator-executable-path-unavailable"))?;
        if metadata.file_type().is_symlink()
            || (component == path && !metadata.is_file())
            || (component != path && !metadata.is_dir())
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(mediator_error("mediator-executable-path-mutable"));
        }
        reject_effective_user_control(&metadata, effective_user_id)?;
        let encoded = std::ffi::CString::new(component.as_os_str().as_bytes())
            .map_err(|_| mediator_error("mediator-executable-path-invalid"))?;
        let access = unsafe {
            libc::faccessat(
                libc::AT_FDCWD,
                encoded.as_ptr(),
                libc::W_OK,
                libc::AT_EACCESS,
            )
        };
        if access == 0 {
            return Err(mediator_error("mediator-executable-path-mutable"));
        }
        let error = std::io::Error::last_os_error();
        if !matches!(
            error.raw_os_error(),
            Some(libc::EACCES) | Some(libc::EPERM) | Some(libc::EROFS)
        ) {
            return Err(mediator_error(
                "mediator-executable-path-access-check-failed",
            ));
        }
        current = component.parent();
    }
    Ok(())
}

#[cfg(unix)]
fn reject_effective_user_control(
    metadata: &fs::Metadata,
    effective_user_id: libc::uid_t,
) -> Result<(), RoutineError> {
    if effective_user_id == 0 || metadata.uid() == effective_user_id {
        Err(mediator_error("mediator-executable-path-mutable"))
    } else {
        Ok(())
    }
}

struct ScopeAnchor {
    relative: RepoPath,
    path: PathBuf,
    file: File,
    #[cfg(unix)]
    identity: ObjectIdentity,
}

pub(super) struct OutputConfinement {
    scopes: Vec<ScopeAnchor>,
    budget_bytes: u64,
}

impl OutputConfinement {
    pub(super) fn prepare(
        root: &RootAnchor,
        scopes: &[RepoPath],
        budget_bytes: u64,
    ) -> Result<Self, RoutineError> {
        #[cfg(not(unix))]
        {
            let _ = (root, scopes, budget_bytes);
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            for (index, left) in scopes.iter().enumerate() {
                if scopes
                    .iter()
                    .skip(index + 1)
                    .any(|right| left.matches_prefix(right) || right.matches_prefix(left))
                {
                    return Err(mediator_error("mediator-output-scope-overlap"));
                }
            }
            let mut anchors = Vec::with_capacity(scopes.len());
            for relative in scopes {
                let path = root.path().join(relative.as_str());
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| mediator_error("mediator-output-scope-missing"))?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(mediator_error("mediator-output-scope-not-directory"));
                }
                let canonical = path
                    .canonicalize()
                    .map_err(|_| mediator_error("mediator-output-scope-unavailable"))?;
                if canonical != path || !canonical.starts_with(root.path()) {
                    return Err(mediator_error("mediator-output-scope-escapes-root"));
                }
                let file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
                    .open(&path)
                    .map_err(|_| mediator_error("mediator-output-scope-open-failed"))?;
                let identity = ObjectIdentity::from(
                    &file
                        .metadata()
                        .map_err(|_| mediator_error("mediator-output-scope-metadata-failed"))?,
                );
                if identity.device != root.device() {
                    return Err(mediator_error("mediator-output-scope-cross-device"));
                }
                anchors.push(ScopeAnchor {
                    relative: relative.clone(),
                    path,
                    file,
                    identity,
                });
            }
            let confinement = Self {
                scopes: anchors,
                budget_bytes,
            };
            confinement.capture()?;
            Ok(confinement)
        }
    }

    pub(super) fn absolute_scopes(&self) -> Vec<&Path> {
        self.scopes
            .iter()
            .map(|scope| scope.path.as_path())
            .collect()
    }

    pub(super) fn capture(&self) -> Result<BTreeMap<String, OutputFileRecord>, RoutineError> {
        let mut files = BTreeMap::new();
        let mut consumed = 0_u64;
        for scope in &self.scopes {
            capture_tree(
                &scope.file,
                &scope.relative,
                scope.identity.device,
                self.budget_bytes,
                &mut consumed,
                &mut files,
            )?;
        }
        Ok(files)
    }

    pub(super) fn validate(&self) -> Result<(), RoutineError> {
        #[cfg(not(unix))]
        {
            return Err(mediator_error("mediator-unix-confinement-required"));
        }
        #[cfg(unix)]
        {
            for scope in &self.scopes {
                let held = ObjectIdentity::from(
                    &scope
                        .file
                        .metadata()
                        .map_err(|_| mediator_error("mediator-output-scope-metadata-failed"))?,
                );
                let current_meta = fs::symlink_metadata(&scope.path)
                    .map_err(|_| mediator_error("mediator-output-scope-replaced"))?;
                if current_meta.file_type().is_symlink() || !current_meta.is_dir() {
                    return Err(mediator_error("mediator-output-scope-replaced"));
                }
                let current = ObjectIdentity::from(&current_meta);
                let canonical = scope
                    .path
                    .canonicalize()
                    .map_err(|_| mediator_error("mediator-output-scope-replaced"))?;
                if !held.same_anchor_object(scope.identity)
                    || !current.same_anchor_object(scope.identity)
                    || canonical != scope.path
                {
                    return Err(RoutineError::new(
                        RoutineErrorId::ConcurrentMutation,
                        "mediator-output-scope-replaced",
                        None,
                    ));
                }
            }
            Ok(())
        }
    }
}

#[cfg(unix)]
fn capture_tree(
    directory: &File,
    relative: &RepoPath,
    device: u64,
    budget: u64,
    consumed: &mut u64,
    files: &mut BTreeMap<String, OutputFileRecord>,
) -> Result<(), RoutineError> {
    let directory_before = ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
    );
    let entries = directory_names(directory)?;
    run_test_capture_hook();
    for name in entries {
        if files.len() >= MAX_OUTPUT_FILES {
            return Err(mediator_error("mediator-output-file-limit-exceeded"));
        }
        let child_relative = RepoPath::parse(format!("{}/{}", relative.as_str(), name))?;
        let encoded = CString::new(name.as_bytes())
            .map_err(|_| mediator_error("mediator-output-path-invalid"))?;
        let mut child = open_child(directory, &encoded)?;
        let metadata = child
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?;
        if (!metadata.is_file() && !metadata.is_dir()) || metadata.dev() != device {
            return Err(mediator_error("mediator-output-object-unsafe"));
        }
        let identity = ObjectIdentity::from(&metadata);
        if metadata.is_dir() {
            capture_tree(&child, &child_relative, device, budget, consumed, files)?;
            require_current_child(directory, &encoded, identity)?;
            continue;
        }
        if metadata.nlink() != 1 {
            return Err(mediator_error("mediator-output-hardlink-refused"));
        }
        *consumed = consumed
            .checked_add(metadata.len())
            .filter(|total| *total <= budget)
            .ok_or_else(|| mediator_error("mediator-output-artifact-budget-exceeded"))?;
        let digest = digest_reader(&mut child, metadata.len())?;
        let after = ObjectIdentity::from(
            &child
                .metadata()
                .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
        );
        if identity != after {
            return Err(RoutineError::new(
                RoutineErrorId::ConcurrentMutation,
                "mediator-output-replaced-during-capture",
                None,
            ));
        }
        require_current_child(directory, &encoded, identity)?;
        files.insert(
            child_relative.as_str().to_owned(),
            OutputFileRecord {
                sha256: digest,
                byte_length: metadata.len(),
                unix_mode: Some(metadata.mode()),
            },
        );
    }
    let directory_after = ObjectIdentity::from(
        &directory
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
    );
    if directory_before != directory_after {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-output-directory-mutated-during-capture",
            None,
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn open_child(directory: &File, name: &CStr) -> Result<File, RoutineError> {
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(mediator_error("mediator-output-object-unsafe"));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(unix)]
fn require_current_child(
    directory: &File,
    name: &CStr,
    expected: ObjectIdentity,
) -> Result<(), RoutineError> {
    let current = open_child(directory, name).map_err(|_| {
        RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-output-replaced-during-capture",
            None,
        )
    })?;
    let current = ObjectIdentity::from(
        &current
            .metadata()
            .map_err(|_| mediator_error("mediator-output-metadata-failed"))?,
    );
    if current != expected {
        return Err(RoutineError::new(
            RoutineErrorId::ConcurrentMutation,
            "mediator-output-replaced-during-capture",
            None,
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn directory_names(directory: &File) -> Result<Vec<String>, RoutineError> {
    let dot = c".";
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            dot.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(mediator_error("mediator-output-directory-open-failed"));
    }
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe {
            libc::close(descriptor);
        }
        return Err(mediator_error("mediator-output-directory-read-failed"));
    }
    let stream = DirectoryStream(stream);
    let mut names = Vec::new();
    loop {
        clear_readdir_error();
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            if readdir_failed() {
                return Err(mediator_error("mediator-output-directory-read-failed"));
            }
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        let bytes = name.to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| mediator_error("mediator-output-path-not-utf8"))?;
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

#[cfg(unix)]
fn clear_readdir_error() {
    #[cfg(target_os = "macos")]
    unsafe {
        *libc::__error() = 0;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        *libc::__errno_location() = 0;
    }
}

#[cfg(unix)]
fn readdir_failed() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        return *libc::__error() != 0;
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        return *libc::__errno_location() != 0;
    }
    #[allow(unreachable_code)]
    false
}

#[cfg(unix)]
struct DirectoryStream(*mut libc::DIR);

#[cfg(unix)]
impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe {
            libc::closedir(self.0);
        }
    }
}

#[cfg(test)]
type CaptureHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
type ReadSourceCaptureHook = Box<dyn FnOnce() + Send + 'static>;

#[cfg(test)]
fn capture_hook() -> &'static Mutex<Option<CaptureHook>> {
    static HOOK: OnceLock<Mutex<Option<CaptureHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn read_source_capture_hook() -> &'static Mutex<Option<ReadSourceCaptureHook>> {
    static HOOK: OnceLock<Mutex<Option<ReadSourceCaptureHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_test_output_capture_hook(hook: impl FnOnce() + Send + 'static) {
    *capture_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
pub(crate) fn set_test_read_source_capture_hook(hook: impl FnOnce() + Send + 'static) {
    *read_source_capture_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
}

#[cfg(test)]
fn run_test_capture_hook() {
    if let Some(hook) = capture_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(test)]
fn run_test_read_source_capture_hook() {
    if let Some(hook) = read_source_capture_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take()
    {
        hook();
    }
}

#[cfg(not(test))]
fn run_test_capture_hook() {}

#[cfg(not(test))]
fn run_test_read_source_capture_hook() {}

fn digest_reader(reader: &mut File, limit: u64) -> Result<String, RoutineError> {
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| mediator_error("mediator-file-read-failed"))?;
        if read == 0 {
            break;
        }
        observed = observed
            .checked_add(read as u64)
            .filter(|total| *total <= limit)
            .ok_or_else(|| mediator_error("mediator-file-read-limit-exceeded"))?;
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn decode_path(value: &str) -> Result<PathBuf, RoutineError> {
    if value.is_empty()
        || value.len() % 2 != 0
        || value.len() > 16_384
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(mediator_error("mediator-executable-path-encoding-invalid"));
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)
                .map_err(|_| mediator_error("mediator-executable-path-encoding-invalid"))?;
            u8::from_str_radix(text, 16)
                .map_err(|_| mediator_error("mediator-executable-path-encoding-invalid"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    #[cfg(unix)]
    {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(OsString::from_vec(bytes)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(bytes)
            .map(PathBuf::from)
            .map_err(|_| mediator_error("mediator-executable-path-encoding-invalid"))
    }
}

fn mediator_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}

#[cfg(all(test, unix))]
mod tests {
    use super::{
        PinnedExecutable, reject_effective_user_control, validate_execution_path_immutability,
    };
    use std::ffi::CString;
    use std::fs;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_EXECUTABLE_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct OwnedExecutionPath {
        root: PathBuf,
        ancestor: PathBuf,
        executable: PathBuf,
    }

    impl OwnedExecutionPath {
        fn new(label: &str) -> Self {
            let sequence = NEXT_EXECUTABLE_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "hul-routine-executable-immutability-{label}-{}-{sequence}",
                std::process::id()
            ));
            let ancestor = root.join("owned-bin");
            let executable = ancestor.join("owned-sh");
            fs::create_dir_all(&ancestor).unwrap();
            fs::write(&executable, b"#!/bin/sh\nexit 0\n").unwrap();
            fs::set_permissions(&ancestor, fs::Permissions::from_mode(0o555)).unwrap();
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o555)).unwrap();
            Self {
                root,
                ancestor,
                executable,
            }
        }
    }

    impl Drop for OwnedExecutionPath {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.ancestor, fs::Permissions::from_mode(0o755));
            let _ = fs::set_permissions(&self.executable, fs::Permissions::from_mode(0o755));
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn current_user_owned_0555_executable_is_mutable_despite_denied_write_access() {
        let fixture = OwnedExecutionPath::new("leaf");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(&fixture.executable).unwrap();
        assert_eq!(metadata.uid(), effective_user_id);
        assert_eq!(metadata.mode() & 0o777, 0o555);
        if effective_user_id != 0 {
            let encoded = CString::new(fixture.executable.as_os_str().as_bytes()).unwrap();
            assert_ne!(unsafe { libc::access(encoded.as_ptr(), libc::W_OK) }, 0);
        }
        let error = validate_execution_path_immutability(&fixture.executable).unwrap_err();
        assert_eq!(error.cause(), "mediator-executable-path-mutable");
    }

    #[test]
    fn current_user_owned_0555_ancestor_and_effective_root_are_mutable() {
        let fixture = OwnedExecutionPath::new("ancestor");
        let effective_user_id = unsafe { libc::geteuid() };
        let ancestor = fs::symlink_metadata(&fixture.ancestor).unwrap();
        assert_eq!(ancestor.uid(), effective_user_id);
        assert_eq!(ancestor.mode() & 0o777, 0o555);
        assert_eq!(
            reject_effective_user_control(&ancestor, effective_user_id)
                .unwrap_err()
                .cause(),
            "mediator-executable-path-mutable"
        );

        let system_shell = fs::symlink_metadata(Path::new("/bin/sh")).unwrap();
        assert_eq!(
            reject_effective_user_control(&system_shell, 0)
                .unwrap_err()
                .cause(),
            "mediator-executable-path-mutable"
        );
    }

    #[test]
    fn root_owned_system_shell_path_remains_eligible_for_non_root_effective_user() {
        let shell = Path::new("/bin/sh");
        let effective_user_id = unsafe { libc::geteuid() };
        let metadata = fs::symlink_metadata(shell).unwrap();
        assert_eq!(metadata.uid(), 0);
        assert_eq!(metadata.mode() & 0o022, 0);
        if effective_user_id == 0 {
            assert_eq!(
                validate_execution_path_immutability(shell)
                    .unwrap_err()
                    .cause(),
                "mediator-executable-path-mutable"
            );
            return;
        }
        assert_ne!(metadata.uid(), effective_user_id);
        validate_execution_path_immutability(shell).unwrap();
        let executable = PinnedExecutable::open_unbound(shell).unwrap();
        executable.validate().unwrap();
    }
}
