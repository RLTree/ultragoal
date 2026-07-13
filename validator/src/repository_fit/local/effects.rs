//! Descriptor-anchored local compare-exchange effects.
//!
//! The supported implementation is intentionally Darwin-only because it
//! relies on `renameatx_np(RENAME_SWAP|RENAME_EXCL)` for one finite atomic
//! namespace linearization point. Other hosts fail closed.

#[cfg(target_vendor = "apple")]
mod supported {
    use super::super::sys::{
        duplicate, exact_entry, open_at, stat_at, EntryMatch, EnumerationBudget, PathStat,
    };
    use crate::repository_fit::product_adapter::LocalMutationGrant;
    use crate::repository_fit::{
        digest, error, CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader,
        LocalRepository,
    };
    use std::collections::BTreeMap;
    #[cfg(test)]
    use std::collections::BTreeSet;
    use std::ffi::{CStr, CString, OsString};
    use std::fs::{self, File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    #[cfg(test)]
    use std::sync::{Arc, Barrier};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

    struct ParentAnchor {
        directory: File,
        canonical: PathBuf,
        attachments: Vec<(File, String, ObjectIdentity)>,
    }

    struct PrivateTransaction {
        name: String,
        directory: File,
        canonical: PathBuf,
        identity: ObjectIdentity,
    }

    #[derive(Clone)]
    struct CreatedDirectory {
        path: String,
        identity: ObjectIdentity,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ObjectIdentity {
        device: u64,
        inode: u64,
    }

    struct ObservedLeaf {
        file: File,
        stat: PathStat,
        bytes: Vec<u8>,
        mode: u32,
    }

    /// Concrete local effect adapter. Construction binds one directory
    /// descriptor and never follows links or consults host configuration.
    pub(crate) struct LocalEffects {
        path: PathBuf,
        canonical: PathBuf,
        root: File,
        root_identity: ObjectIdentity,
        binding: String,
        reader: LocalRepository,
        mutation_lease: bool,
        unix_modes: BTreeMap<String, u32>,
        rollback_modes: BTreeMap<String, (String, u32)>,
        created_directories: BTreeMap<String, ObjectIdentity>,
        #[cfg(test)]
        fail_calls: BTreeSet<usize>,
        #[cfg(test)]
        compare_calls: usize,
        #[cfg(test)]
        before_linearize: Option<(Arc<Barrier>, Arc<Barrier>)>,
        #[cfg(test)]
        after_replace_swap: Option<(Arc<Barrier>, Arc<Barrier>)>,
        #[cfg(test)]
        before_quarantine_move: Option<(Arc<Barrier>, Arc<Barrier>)>,
        #[cfg(test)]
        after_directory_publish: Option<(Arc<Barrier>, Arc<Barrier>)>,
        #[cfg(test)]
        before_directory_quarantine_move: Option<(Arc<Barrier>, Arc<Barrier>)>,
    }

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
        pub(crate) fn test_device_guard(
            root_device: u64,
            object_device: u64,
        ) -> Result<(), FitError> {
            require_same_device(root_device, object_device)
        }

        #[cfg(test)]
        pub(crate) fn pause_before_linearize(
            &mut self,
            reached: Arc<Barrier>,
            resume: Arc<Barrier>,
        ) {
            self.before_linearize = Some((reached, resume));
        }

        #[cfg(test)]
        pub(crate) fn pause_after_replace_swap(
            &mut self,
            reached: Arc<Barrier>,
            resume: Arc<Barrier>,
        ) {
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

        pub(crate) fn read_unix_mode(
            &mut self,
            path: &CanonicalPath,
        ) -> Result<Option<u32>, FitError> {
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

        fn verify_root(&self) -> Result<(), FitError> {
            let path =
                fs::symlink_metadata(&self.path).map_err(|_| error(FitErrorId::StaleBinding))?;
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

        fn parent_anchor(
            &mut self,
            path: &CanonicalPath,
            create: bool,
        ) -> Result<(ParentAnchor, Vec<CreatedDirectory>), FitError> {
            self.verify_root()?;
            let components = path.components().collect::<Vec<_>>();
            let mut directory = duplicate(&self.root)?;
            let mut canonical = self.canonical.clone();
            let mut relative = Vec::<String>::new();
            let mut attachments = Vec::new();
            let mut created = Vec::new();
            let mut budget = EnumerationBudget::new();
            for component in &components[..components.len() - 1] {
                let classification = exact_entry(&directory, component, &mut budget)?;
                match classification {
                    EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
                    EntryMatch::Absent if !create => return Err(error(FitErrorId::Conflict)),
                    EntryMatch::Absent => {
                        let (next, identity) =
                            self.create_directory(&directory, &canonical, component)?;
                        relative.push((*component).to_owned());
                        created.push(CreatedDirectory {
                            path: relative.join("/"),
                            identity,
                        });
                        attachments.push((
                            duplicate(&directory)?,
                            (*component).to_owned(),
                            identity,
                        ));
                        canonical.push(component);
                        directory = next;
                        continue;
                    }
                    EntryMatch::Exact => relative.push((*component).to_owned()),
                }
                let before = stat_at(&directory, component)?
                    .ok_or_else(|| error(FitErrorId::StaleBinding))?;
                let next = open_at(
                    &directory,
                    component,
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )?
                .ok_or_else(|| error(FitErrorId::StaleBinding))?;
                let opened = next.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
                if !opened.is_dir()
                    || require_same_device(self.root_identity.device, opened.dev()).is_err()
                    || object_identity(&opened) != stat_identity(before)
                    || exact_entry(&directory, component, &mut budget)? != EntryMatch::Exact
                {
                    return Err(error(if created.is_empty() {
                        FitErrorId::UnsafeObject
                    } else {
                        FitErrorId::RollbackFailed
                    }));
                }
                attachments.push((
                    duplicate(&directory)?,
                    (*component).to_owned(),
                    object_identity(&opened),
                ));
                canonical.push(component);
                if descriptor_path(&next)? != canonical {
                    return Err(error(if created.is_empty() {
                        FitErrorId::StaleBinding
                    } else {
                        FitErrorId::RollbackFailed
                    }));
                }
                directory = next;
            }
            let anchor = ParentAnchor {
                directory,
                canonical,
                attachments,
            };
            self.verify_parent(&anchor)?;
            Ok((anchor, created))
        }

        fn verify_parent(&self, parent: &ParentAnchor) -> Result<(), FitError> {
            self.verify_root()?;
            let metadata = parent
                .directory
                .metadata()
                .map_err(|_| error(FitErrorId::StaleBinding))?;
            if !metadata.is_dir()
                || require_same_device(self.root_identity.device, metadata.dev()).is_err()
                || descriptor_path(&parent.directory)? != parent.canonical
            {
                return Err(error(FitErrorId::StaleBinding));
            }
            let mut budget = EnumerationBudget::new();
            for (directory, name, expected) in parent.attachments.iter().rev() {
                let stat =
                    stat_at(directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
                if exact_entry(directory, name, &mut budget)? != EntryMatch::Exact
                    || stat_identity(stat) != *expected
                {
                    return Err(error(FitErrorId::StaleBinding));
                }
            }
            Ok(())
        }

        fn create_directory(
            &mut self,
            parent: &File,
            parent_canonical: &Path,
            name: &str,
        ) -> Result<(File, ObjectIdentity), FitError> {
            let staged_name = directory_temp_name()?;
            let mut budget = EnumerationBudget::new();
            if exact_entry(parent, &staged_name, &mut budget)? != EntryMatch::Absent {
                return Err(error(FitErrorId::UnsafeObject));
            }
            mkdir_at(parent, &staged_name, 0o755)?;
            let staged = open_at(
                parent,
                &staged_name,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?
            .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
            let metadata = staged
                .metadata()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            let identity = object_identity(&metadata);
            if !metadata.is_dir()
                || require_same_device(self.root_identity.device, metadata.dev()).is_err()
                || unsafe { libc::fchmod(staged.as_raw_fd(), 0o755 as libc::mode_t) } != 0
                || staged.sync_all().is_err()
                || stat_at(parent, &staged_name)?.is_none_or(|stat| stat_identity(stat) != identity)
                || descriptor_path(&staged).ok() != Some(parent_canonical.join(&staged_name))
            {
                return Err(error(FitErrorId::RollbackFailed));
            }
            if let Err(failure) = rename_exclusive(parent, &staged_name, name) {
                let cleaned = self.quarantine_remove_directory_from(
                    parent,
                    parent_canonical,
                    &staged_name,
                    identity,
                    &staged,
                )?;
                return Err(error(if cleaned {
                    failure.id()
                } else {
                    FitErrorId::RollbackFailed
                }));
            }
            self.test_after_directory_publish();
            let current = stat_at(parent, name)?;
            if current.is_none_or(|stat| stat_identity(stat) != identity)
                || descriptor_path(&staged).ok() != Some(parent_canonical.join(name))
                || exact_entry(parent, name, &mut budget)? != EntryMatch::Exact
            {
                return Err(error(FitErrorId::RollbackFailed));
            }
            Ok((staged, identity))
        }

        fn observe_leaf(
            &self,
            parent: &ParentAnchor,
            name: &str,
        ) -> Result<Option<ObservedLeaf>, FitError> {
            self.verify_parent(parent)?;
            let mut budget = EnumerationBudget::new();
            match exact_entry(&parent.directory, name, &mut budget)? {
                EntryMatch::Absent => {
                    if stat_at(&parent.directory, name)?.is_some() {
                        return Err(error(FitErrorId::StaleBinding));
                    }
                    return Ok(None);
                }
                EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
                EntryMatch::Exact => {}
            }
            let before =
                stat_at(&parent.directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
            if !before.regular
                || before.links != 1
                || require_same_device(self.root_identity.device, before.device).is_err()
                || before.length > crate::repository_fit::model::MAX_FILE_BYTES as u64
            {
                return Err(error(
                    if before.length > crate::repository_fit::model::MAX_FILE_BYTES as u64 {
                        FitErrorId::ResourceLimit
                    } else {
                        FitErrorId::UnsafeObject
                    },
                ));
            }
            let mut file = open_at(
                &parent.directory,
                name,
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )?
            .ok_or_else(|| error(FitErrorId::StaleBinding))?;
            let metadata = file.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
            if leaf_stat(&metadata) != before {
                return Err(error(FitErrorId::StaleBinding));
            }
            let first = bounded_read(&mut file)?;
            file.seek(SeekFrom::Start(0))
                .map_err(|_| error(FitErrorId::ReadFailed))?;
            let second = bounded_read(&mut file)?;
            let after =
                stat_at(&parent.directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
            let expected_path = parent.canonical.join(name);
            if first != second
                || first.len() as u64 != before.length
                || before != after
                || exact_entry(&parent.directory, name, &mut budget)? != EntryMatch::Exact
                || descriptor_path(&file)? != expected_path
            {
                return Err(error(FitErrorId::StaleBinding));
            }
            self.verify_parent(parent)?;
            Ok(Some(ObservedLeaf {
                file,
                stat: before,
                bytes: first,
                mode: metadata.permissions().mode() & 0o7777,
            }))
        }

        fn create_replacement(
            &mut self,
            parent: &ParentAnchor,
            _name: &str,
            bytes: &[u8],
            mode: u32,
        ) -> Result<(String, File, ObjectIdentity), FitError> {
            let temp_name = temp_name();
            let mut budget = EnumerationBudget::new();
            if exact_entry(&parent.directory, &temp_name, &mut budget)? != EntryMatch::Absent {
                return Err(error(FitErrorId::UnsafeObject));
            }
            let mut file = create_file_at(&parent.directory, &temp_name, mode)?;
            let write_succeeded = unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) }
                == 0
                && file.write_all(bytes).and_then(|()| file.sync_all()).is_ok();
            let metadata = file
                .metadata()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            let identity = object_identity(&metadata);
            if !write_succeeded
                || !metadata.is_file()
                || metadata.nlink() != 1
                || require_same_device(self.root_identity.device, metadata.dev()).is_err()
                || metadata.len() != bytes.len() as u64
                || metadata.permissions().mode() & 0o7777 != mode
                || descriptor_path(&file).ok() != Some(parent.canonical.join(&temp_name))
            {
                return Err(error(
                    if self
                        .quarantine_unlink(parent, &temp_name, identity, &file)
                        .is_ok()
                    {
                        FitErrorId::EffectFailed
                    } else {
                        FitErrorId::RollbackFailed
                    },
                ));
            }
            Ok((temp_name, file, identity))
        }

        fn insert_absent(
            &mut self,
            parent: &ParentAnchor,
            name: &str,
            replacement: &[u8],
            created: Vec<CreatedDirectory>,
            mode: u32,
        ) -> Result<bool, FitError> {
            let (temp, file, identity) =
                self.create_replacement(parent, name, replacement, mode)?;
            self.test_before_linearize();
            match rename_exclusive(&parent.directory, &temp, name) {
                Ok(()) => {}
                Err(failure) if failure.id() == FitErrorId::Conflict => {
                    self.quarantine_unlink(parent, &temp, identity, &file)?;
                    self.cleanup_paths(&created)?;
                    return Ok(false);
                }
                Err(failure) => {
                    self.quarantine_unlink(parent, &temp, identity, &file)?;
                    self.cleanup_paths(&created)?;
                    return Err(failure);
                }
            }
            let active = stat_at(&parent.directory, name)?;
            if active.is_none_or(|stat| stat_identity(stat) != identity)
                || descriptor_path(&file)? != parent.canonical.join(name)
                || self.verify_parent(parent).is_err()
            {
                return Err(error(FitErrorId::RollbackFailed));
            }
            self.created_directories
                .extend(created.into_iter().map(|row| (row.path, row.identity)));
            Ok(true)
        }

        fn replace_existing(
            &mut self,
            parent: &ParentAnchor,
            name: &str,
            observed: ObservedLeaf,
            replacement: &[u8],
            replacement_mode: u32,
        ) -> Result<bool, FitError> {
            let (temp, next, next_identity) =
                self.create_replacement(parent, name, replacement, replacement_mode)?;
            self.test_before_linearize();
            if let Err(failure) = rename_swap(&parent.directory, &temp, name) {
                self.quarantine_unlink(parent, &temp, next_identity, &next)?;
                return Err(failure);
            }
            self.test_after_replace_swap();
            let swapped_prior = stat_at(&parent.directory, &temp)?;
            let active = stat_at(&parent.directory, name)?;
            let valid = swapped_prior
                .is_some_and(|stat| stat_identity(stat) == stat_identity(observed.stat))
                && active.is_some_and(|stat| stat_identity(stat) == next_identity)
                && descriptor_path(&observed.file).ok() == Some(parent.canonical.join(&temp))
                && descriptor_path(&next).ok() == Some(parent.canonical.join(name))
                && self.verify_parent(parent).is_ok();
            if !valid {
                return Err(error(FitErrorId::RollbackFailed));
            }
            self.quarantine_unlink(parent, &temp, stat_identity(observed.stat), &observed.file)?;
            Ok(true)
        }

        fn remove_existing(
            &mut self,
            parent: &ParentAnchor,
            name: &str,
            observed: ObservedLeaf,
        ) -> Result<bool, FitError> {
            let temp = temp_name();
            self.test_before_linearize();
            rename_exclusive(&parent.directory, name, &temp)?;
            let moved = stat_at(&parent.directory, &temp)?;
            let valid = moved
                .is_some_and(|stat| stat_identity(stat) == stat_identity(observed.stat))
                && stat_at(&parent.directory, name)?.is_none()
                && descriptor_path(&observed.file).ok() == Some(parent.canonical.join(&temp))
                && self.verify_parent(parent).is_ok();
            if !valid {
                return Err(error(FitErrorId::RollbackFailed));
            }
            self.quarantine_unlink(parent, &temp, stat_identity(observed.stat), &observed.file)?;
            self.cleanup_created_directories()?;
            self.verify_root()?;
            Ok(true)
        }

        fn quarantine_unlink(
            &mut self,
            source_parent: &ParentAnchor,
            source_name: &str,
            expected: ObjectIdentity,
            held: &File,
        ) -> Result<(), FitError> {
            let transaction = self.open_private_transaction()?;
            self.test_before_quarantine_move();
            if rename_between(
                &source_parent.directory,
                source_name,
                &transaction.directory,
                "object",
                libc::RENAME_EXCL,
            )
            .is_err()
            {
                let _ = self.close_private_transaction(transaction);
                return Err(error(FitErrorId::RollbackFailed));
            }
            let mut budget = EnumerationBudget::new();
            let moved = stat_at(&transaction.directory, "object")?;
            let valid = moved.is_some_and(|stat| stat_identity(stat) == expected)
                && exact_entry(&transaction.directory, "object", &mut budget)? == EntryMatch::Exact
                && descriptor_path(held).ok() == Some(transaction.canonical.join("object"))
                && self.private_transaction_is_current(&transaction).is_ok();
            if !valid {
                return Err(error(FitErrorId::RollbackFailed));
            }
            unlink_at(&transaction.directory, "object", 0)
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            transaction
                .directory
                .sync_all()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            self.close_private_transaction(transaction)
        }

        fn quarantine_remove_directory(
            &mut self,
            source_parent: &ParentAnchor,
            source_name: &str,
            expected: ObjectIdentity,
        ) -> Result<bool, FitError> {
            let held = open_at(
                &source_parent.directory,
                source_name,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?
            .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
            self.quarantine_remove_directory_from(
                &source_parent.directory,
                &source_parent.canonical,
                source_name,
                expected,
                &held,
            )
        }

        fn quarantine_remove_directory_from(
            &mut self,
            source_parent: &File,
            source_parent_canonical: &Path,
            source_name: &str,
            expected: ObjectIdentity,
            held: &File,
        ) -> Result<bool, FitError> {
            let metadata = held
                .metadata()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            let mut budget = EnumerationBudget::new();
            if !metadata.is_dir()
                || object_identity(&metadata) != expected
                || descriptor_path(held).ok() != Some(source_parent_canonical.join(source_name))
                || exact_entry(source_parent, source_name, &mut budget)? != EntryMatch::Exact
                || stat_at(source_parent, source_name)?
                    .is_none_or(|stat| stat_identity(stat) != expected)
            {
                return Err(error(FitErrorId::RollbackFailed));
            }
            if !directory_is_empty(held)? {
                return Ok(false);
            }
            let transaction = self.open_private_transaction()?;
            self.test_before_directory_quarantine_move();
            if rename_between(
                source_parent,
                source_name,
                &transaction.directory,
                "object",
                libc::RENAME_EXCL,
            )
            .is_err()
            {
                let _ = self.close_private_transaction(transaction);
                return Err(error(FitErrorId::RollbackFailed));
            }
            let moved = stat_at(&transaction.directory, "object")?;
            let mut budget = EnumerationBudget::new();
            let valid = moved.is_some_and(|stat| stat_identity(stat) == expected)
                && exact_entry(&transaction.directory, "object", &mut budget)? == EntryMatch::Exact
                && descriptor_path(held).ok() == Some(transaction.canonical.join("object"))
                && self.private_transaction_is_current(&transaction).is_ok();
            if !valid {
                return Err(error(FitErrorId::RollbackFailed));
            }
            unlink_at(&transaction.directory, "object", libc::AT_REMOVEDIR)
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            transaction
                .directory
                .sync_all()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            self.close_private_transaction(transaction)?;
            Ok(true)
        }

        fn open_private_transaction(&self) -> Result<PrivateTransaction, FitError> {
            self.verify_root()?;
            for _ in 0..8 {
                let name = transaction_name()?;
                let mut budget = EnumerationBudget::new();
                match exact_entry(&self.root, &name, &mut budget)? {
                    EntryMatch::Absent => {}
                    EntryMatch::Exact | EntryMatch::Alias => continue,
                }
                match mkdir_at(&self.root, &name, 0o700) {
                    Ok(()) => {}
                    Err(failure) if failure.id() == FitErrorId::Conflict => continue,
                    Err(failure) => return Err(failure),
                }
                let directory = open_at(
                    &self.root,
                    &name,
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )?
                .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
                let metadata = directory
                    .metadata()
                    .map_err(|_| error(FitErrorId::RollbackFailed))?;
                let identity = object_identity(&metadata);
                let canonical = self.canonical.join(&name);
                let stat = stat_at(&self.root, &name)?;
                if !metadata.is_dir()
                    || require_same_device(self.root_identity.device, metadata.dev()).is_err()
                    || stat.is_none_or(|stat| stat_identity(stat) != identity)
                    || descriptor_path(&directory).ok() != Some(canonical.clone())
                    || unsafe { libc::fchmod(directory.as_raw_fd(), 0o700 as libc::mode_t) } != 0
                    || directory.sync_all().is_err()
                {
                    return Err(error(FitErrorId::RollbackFailed));
                }
                let transaction = PrivateTransaction {
                    name,
                    directory,
                    canonical,
                    identity,
                };
                self.private_transaction_is_current(&transaction)?;
                return Ok(transaction);
            }
            Err(error(FitErrorId::ResourceLimit))
        }

        fn private_transaction_is_current(
            &self,
            transaction: &PrivateTransaction,
        ) -> Result<(), FitError> {
            self.verify_root()?;
            let mut budget = EnumerationBudget::new();
            let stat = stat_at(&self.root, &transaction.name)?
                .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
            if exact_entry(&self.root, &transaction.name, &mut budget)? != EntryMatch::Exact
                || stat_identity(stat) != transaction.identity
                || descriptor_path(&transaction.directory).ok()
                    != Some(transaction.canonical.clone())
            {
                return Err(error(FitErrorId::RollbackFailed));
            }
            Ok(())
        }

        fn close_private_transaction(
            &self,
            transaction: PrivateTransaction,
        ) -> Result<(), FitError> {
            self.private_transaction_is_current(&transaction)?;
            unlink_at(&self.root, &transaction.name, libc::AT_REMOVEDIR)
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            self.root
                .sync_all()
                .map_err(|_| error(FitErrorId::RollbackFailed))?;
            Ok(())
        }

        fn cleanup_created_directories(&mut self) -> Result<(), FitError> {
            let mut paths = self
                .created_directories
                .iter()
                .map(|(path, identity)| CreatedDirectory {
                    path: path.clone(),
                    identity: *identity,
                })
                .collect::<Vec<_>>();
            paths.sort_by_key(|row| std::cmp::Reverse(row.path.matches('/').count()));
            for row in paths {
                match self.remove_directory(&row) {
                    Ok(true) => {
                        self.created_directories.remove(&row.path);
                    }
                    Ok(false) => {}
                    Err(failure) => return Err(failure),
                }
            }
            Ok(())
        }

        fn cleanup_paths(&mut self, paths: &[CreatedDirectory]) -> Result<(), FitError> {
            for row in paths.iter().rev() {
                let _ = self.remove_directory(row)?;
            }
            Ok(())
        }

        fn remove_directory(&mut self, created: &CreatedDirectory) -> Result<bool, FitError> {
            let marker = match created.path.rsplit_once('/') {
                Some((parent, _)) => CanonicalPath::parse(format!("{parent}/.hul-cleanup"))?,
                None => CanonicalPath::parse(".hul-cleanup")?,
            };
            let (parent, _) = self.parent_anchor(&marker, false)?;
            let name = created
                .path
                .rsplit('/')
                .next()
                .ok_or_else(|| error(FitErrorId::InvalidPath))?;
            let mut budget = EnumerationBudget::new();
            if exact_entry(&parent.directory, name, &mut budget)? != EntryMatch::Exact {
                return Err(error(FitErrorId::StaleBinding));
            }
            let stat = stat_at(&parent.directory, name)?
                .ok_or_else(|| error(FitErrorId::RollbackFailed))?;
            if stat_identity(stat) != created.identity {
                return Err(error(FitErrorId::RollbackFailed));
            }
            self.quarantine_remove_directory(&parent, name, created.identity)
        }

        fn desired_mode(&self, path: &CanonicalPath) -> Result<u32, FitError> {
            self.unix_modes
                .get(path.as_str())
                .copied()
                .ok_or_else(|| error(FitErrorId::InvalidSpec))
        }

        #[cfg(test)]
        fn injected_failure(&mut self) -> Result<(), FitError> {
            self.compare_calls += 1;
            if self.fail_calls.contains(&self.compare_calls) {
                Err(error(FitErrorId::EffectFailed))
            } else {
                Ok(())
            }
        }

        #[cfg(not(test))]
        const fn injected_failure(&mut self) -> Result<(), FitError> {
            Ok(())
        }

        #[cfg(test)]
        fn test_before_linearize(&mut self) {
            if let Some((reached, resume)) = self.before_linearize.take() {
                reached.wait();
                resume.wait();
            }
        }

        #[cfg(not(test))]
        const fn test_before_linearize(&mut self) {}

        #[cfg(test)]
        fn test_after_replace_swap(&mut self) {
            if let Some((reached, resume)) = self.after_replace_swap.take() {
                reached.wait();
                resume.wait();
            }
        }

        #[cfg(not(test))]
        const fn test_after_replace_swap(&mut self) {}

        #[cfg(test)]
        fn test_before_quarantine_move(&mut self) {
            if let Some((reached, resume)) = self.before_quarantine_move.take() {
                reached.wait();
                resume.wait();
            }
        }

        #[cfg(not(test))]
        const fn test_before_quarantine_move(&mut self) {}

        #[cfg(test)]
        fn test_after_directory_publish(&mut self) {
            if let Some((reached, resume)) = self.after_directory_publish.take() {
                reached.wait();
                resume.wait();
            }
        }

        #[cfg(not(test))]
        const fn test_after_directory_publish(&mut self) {}

        #[cfg(test)]
        fn test_before_directory_quarantine_move(&mut self) {
            if let Some((reached, resume)) = self.before_directory_quarantine_move.take() {
                reached.wait();
                resume.wait();
            }
        }

        #[cfg(not(test))]
        const fn test_before_directory_quarantine_move(&mut self) {}
    }

    impl FitReader for LocalEffects {
        fn root_binding(&mut self) -> Result<String, FitError> {
            self.verify_root()?;
            if self.reader.root_binding()? != self.binding {
                return Err(error(FitErrorId::StaleBinding));
            }
            Ok(self.binding.clone())
        }

        fn read_file(
            &mut self,
            path: &CanonicalPath,
            maximum_bytes: usize,
        ) -> Result<Option<Vec<u8>>, FitError> {
            self.verify_root()?;
            self.reader.read_file(path, maximum_bytes)
        }
    }

    impl FitEffects for LocalEffects {
        fn compare_exchange(
            &mut self,
            path: &CanonicalPath,
            expected: &ExpectedContent,
            replacement: Option<&[u8]>,
        ) -> Result<bool, FitError> {
            if !self.mutation_lease {
                return Err(error(FitErrorId::Unauthorized));
            }
            self.injected_failure()?;
            if replacement
                .is_some_and(|bytes| bytes.len() > crate::repository_fit::model::MAX_FILE_BYTES)
                || matches!((expected, replacement), (ExpectedContent::Absent, None))
            {
                return Err(error(FitErrorId::InvalidSpec));
            }
            let create_parents =
                matches!(expected, ExpectedContent::Absent) && replacement.is_some();
            let (parent, created) = self.parent_anchor(path, create_parents)?;
            let name = path
                .components()
                .last()
                .expect("canonical path is nonempty");
            let observed = self.observe_leaf(&parent, name)?;
            let matches = match (expected, observed.as_ref()) {
                (ExpectedContent::Absent, None) => true,
                (ExpectedContent::ExactDigest(expected), Some(observed)) => {
                    digest(&observed.bytes) == *expected
                }
                _ => false,
            };
            if !matches {
                self.cleanup_paths(&created)?;
                return Ok(false);
            }
            match (observed, replacement) {
                (None, Some(bytes)) => {
                    self.insert_absent(&parent, name, bytes, created, self.desired_mode(path)?)
                }
                (Some(observed), Some(bytes)) => {
                    debug_assert!(created.is_empty());
                    let prior_mode = observed.mode;
                    let rollback = match expected {
                        ExpectedContent::ExactDigest(expected) => self
                            .rollback_modes
                            .get(path.as_str())
                            .filter(|(current, _)| current == expected)
                            .cloned(),
                        ExpectedContent::Absent => None,
                    };
                    let replacement_mode = rollback
                        .as_ref()
                        .map(|(_, mode)| *mode)
                        .or_else(|| self.unix_modes.get(path.as_str()).copied())
                        .ok_or_else(|| error(FitErrorId::InvalidSpec))?;
                    let changed =
                        self.replace_existing(&parent, name, observed, bytes, replacement_mode)?;
                    if changed {
                        if rollback.is_some() {
                            self.rollback_modes.remove(path.as_str());
                        } else {
                            self.rollback_modes
                                .insert(path.as_str().to_owned(), (digest(bytes), prior_mode));
                        }
                    }
                    Ok(changed)
                }
                (Some(observed), None) => {
                    debug_assert!(created.is_empty());
                    self.remove_existing(&parent, name, observed)
                }
                (None, None) => Err(error(FitErrorId::InvalidSpec)),
            }
        }
    }

    fn bounded_read(file: &mut File) -> Result<Vec<u8>, FitError> {
        let maximum = crate::repository_fit::model::MAX_FILE_BYTES;
        let mut bytes = Vec::new();
        Read::by_ref(file)
            .take(maximum as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| error(FitErrorId::ReadFailed))?;
        if bytes.len() > maximum {
            return Err(error(FitErrorId::ResourceLimit));
        }
        Ok(bytes)
    }

    fn object_identity(metadata: &fs::Metadata) -> ObjectIdentity {
        ObjectIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }

    fn require_same_device(root_device: u64, object_device: u64) -> Result<(), FitError> {
        if root_device == object_device {
            Ok(())
        } else {
            Err(error(FitErrorId::UnsafeObject))
        }
    }

    const fn stat_identity(stat: PathStat) -> ObjectIdentity {
        ObjectIdentity {
            device: stat.device,
            inode: stat.inode,
        }
    }

    fn leaf_stat(metadata: &fs::Metadata) -> PathStat {
        PathStat {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            regular: metadata.is_file(),
        }
    }

    fn temp_name() -> String {
        format!(
            ".hul-fit-{:x}-{:x}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        )
    }

    fn transaction_name() -> Result<String, FitError> {
        random_name(".hul-fit-transaction-")
    }

    fn directory_temp_name() -> Result<String, FitError> {
        random_name(".hul-fit-directory-")
    }

    fn random_name(prefix: &str) -> Result<String, FitError> {
        let mut random = [0u8; 16];
        if unsafe {
            libc::getentropy(
                random.as_mut_ptr().cast::<libc::c_void>(),
                random.len() as libc::size_t,
            )
        } != 0
        {
            return Err(error(FitErrorId::UnsupportedHost));
        }
        let mut encoded = String::with_capacity(32);
        for byte in random {
            use std::fmt::Write as _;
            write!(&mut encoded, "{byte:02x}").map_err(|_| error(FitErrorId::EffectFailed))?;
        }
        Ok(format!("{prefix}{encoded}"))
    }

    fn directory_is_empty(directory: &File) -> Result<bool, FitError> {
        let descriptor = unsafe { libc::fcntl(directory.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
        if descriptor < 0 {
            return Err(error(FitErrorId::ReadFailed));
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            unsafe { libc::close(descriptor) };
            return Err(error(FitErrorId::ReadFailed));
        }
        unsafe { libc::rewinddir(stream) };
        let mut entries = 0usize;
        let mut name_bytes = 0usize;
        let result = loop {
            unsafe { *libc::__error() = 0 };
            let entry = unsafe { libc::readdir(stream) };
            if entry.is_null() {
                break if unsafe { *libc::__error() } == 0 {
                    Ok(true)
                } else {
                    Err(error(FitErrorId::ReadFailed))
                };
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            entries = entries.saturating_add(1);
            name_bytes = name_bytes.saturating_add(name.len());
            if entries > 256 || name_bytes > 64 * 1024 {
                break Err(error(FitErrorId::ResourceLimit));
            }
            if !matches!(name, b"." | b"..") {
                break Ok(false);
            }
        };
        let closed = unsafe { libc::closedir(stream) };
        if closed != 0 {
            Err(error(FitErrorId::ReadFailed))
        } else {
            result
        }
    }

    fn mkdir_at(directory: &File, name: &str, mode: u32) -> Result<(), FitError> {
        let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
        if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), mode as libc::mode_t) } == 0
        {
            Ok(())
        } else {
            Err(error(
                match std::io::Error::last_os_error().raw_os_error() {
                    Some(libc::EEXIST) => FitErrorId::Conflict,
                    _ => FitErrorId::EffectFailed,
                },
            ))
        }
    }

    fn create_file_at(directory: &File, name: &str, mode: u32) -> Result<File, FitError> {
        let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                mode,
            )
        };
        if descriptor < 0 {
            return Err(error(FitErrorId::EffectFailed));
        }
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }

    fn rename_exclusive(directory: &File, source: &str, target: &str) -> Result<(), FitError> {
        rename(directory, source, target, libc::RENAME_EXCL)
    }

    fn rename_swap(directory: &File, source: &str, target: &str) -> Result<(), FitError> {
        rename(directory, source, target, libc::RENAME_SWAP)
    }

    fn rename(directory: &File, source: &str, target: &str, flags: u32) -> Result<(), FitError> {
        rename_between(directory, source, directory, target, flags)
    }

    fn rename_between(
        source_directory: &File,
        source: &str,
        target_directory: &File,
        target: &str,
        flags: u32,
    ) -> Result<(), FitError> {
        let source = CString::new(source).map_err(|_| error(FitErrorId::InvalidPath))?;
        let target = CString::new(target).map_err(|_| error(FitErrorId::InvalidPath))?;
        if unsafe {
            libc::renameatx_np(
                source_directory.as_raw_fd(),
                source.as_ptr(),
                target_directory.as_raw_fd(),
                target.as_ptr(),
                flags,
            )
        } == 0
        {
            Ok(())
        } else {
            Err(error(
                match std::io::Error::last_os_error().raw_os_error() {
                    Some(libc::EEXIST | libc::ENOENT) => FitErrorId::Conflict,
                    Some(libc::EXDEV) => FitErrorId::UnsafeObject,
                    Some(libc::ENOTSUP | libc::ENOSYS) => FitErrorId::UnsupportedHost,
                    _ => FitErrorId::EffectFailed,
                },
            ))
        }
    }

    fn unlink_at(directory: &File, name: &str, flags: i32) -> Result<(), FitError> {
        let name = CString::new(name).map_err(|_| error(FitErrorId::InvalidPath))?;
        if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), flags) } == 0 {
            Ok(())
        } else {
            Err(error(
                match std::io::Error::last_os_error().raw_os_error() {
                    Some(libc::ENOENT | libc::ENOTEMPTY | libc::EEXIST) => FitErrorId::Conflict,
                    _ => FitErrorId::EffectFailed,
                },
            ))
        }
    }

    fn descriptor_path(file: &File) -> Result<PathBuf, FitError> {
        let mut buffer = [0 as libc::c_char; libc::PATH_MAX as usize];
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
            return Err(error(
                match std::io::Error::last_os_error().raw_os_error() {
                    Some(libc::EINVAL | libc::ENOTSUP | libc::ENOSYS) => {
                        FitErrorId::UnsupportedHost
                    }
                    _ => FitErrorId::StaleBinding,
                },
            ));
        }
        let end = buffer
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| error(FitErrorId::StaleBinding))?;
        let bytes = buffer[..end]
            .iter()
            .map(|byte| *byte as u8)
            .collect::<Vec<_>>();
        if bytes.first() != Some(&b'/') {
            return Err(error(FitErrorId::StaleBinding));
        }
        Ok(PathBuf::from(OsString::from_vec(bytes)))
    }
}

#[cfg(not(target_vendor = "apple"))]
mod supported {
    use crate::repository_fit::product_adapter::LocalMutationGrant;
    use crate::repository_fit::{
        error, CanonicalPath, ExpectedContent, FitEffects, FitError, FitErrorId, FitReader,
    };
    use std::collections::BTreeMap;
    use std::path::Path;

    pub(crate) struct LocalEffects;

    impl LocalEffects {
        pub(crate) fn open(
            _root: impl AsRef<Path>,
            _unix_modes: BTreeMap<String, u32>,
        ) -> Result<Self, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }

        pub(in crate::repository_fit) fn open_with_mutation_grant(
            _root: impl AsRef<Path>,
            _unix_modes: BTreeMap<String, u32>,
            _grant: LocalMutationGrant,
        ) -> Result<Self, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }

        #[cfg(test)]
        pub(crate) fn open_for_test(
            _root: impl AsRef<Path>,
            _unix_modes: BTreeMap<String, u32>,
        ) -> Result<Self, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }

        pub(crate) fn read_unix_mode(
            &mut self,
            _path: &CanonicalPath,
        ) -> Result<Option<u32>, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }
    }

    impl FitReader for LocalEffects {
        fn root_binding(&mut self) -> Result<String, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }

        fn read_file(
            &mut self,
            _path: &CanonicalPath,
            _maximum_bytes: usize,
        ) -> Result<Option<Vec<u8>>, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }
    }

    impl FitEffects for LocalEffects {
        fn compare_exchange(
            &mut self,
            _path: &CanonicalPath,
            _expected: &ExpectedContent,
            _replacement: Option<&[u8]>,
        ) -> Result<bool, FitError> {
            Err(error(FitErrorId::UnsupportedHost))
        }
    }
}

pub(crate) use supported::LocalEffects;
