use super::super::{CanonicalPath, FitError, FitErrorId, error};
use super::sys::{
    EntryMatch, EnumerationBudget, PathStat, duplicate, exact_entry, open_at, stat_at,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
#[path = "apple_path.rs"]
mod platform_path;
use platform_path::descriptor_path;
pub(super) struct Workspace {
    path: PathBuf,
    canonical: PathBuf,
    root: File,
    device: u64,
    inode: u64,
    binding: String,
}

impl Workspace {
    pub(super) fn open(path: &Path) -> Result<Self, FitError> {
        let path_metadata =
            fs::symlink_metadata(path).map_err(|_| error(FitErrorId::ReadFailed))?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(error(FitErrorId::UnsafeObject));
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let root = options
            .open(path)
            .map_err(|_| error(FitErrorId::UnsafeObject))?;
        let metadata = root.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
        if !metadata.is_dir()
            || metadata.dev() != path_metadata.dev()
            || metadata.ino() != path_metadata.ino()
        {
            return Err(error(FitErrorId::UnsafeObject));
        }
        let canonical = fs::canonicalize(path).map_err(|_| error(FitErrorId::ReadFailed))?;
        let binding =
            super::root_binding_from_canonical_path(&canonical, metadata.dev(), metadata.ino());
        let value = Self {
            path: path.to_path_buf(),
            canonical,
            root,
            device: metadata.dev(),
            inode: metadata.ino(),
            binding,
        };
        value.verify_root()?;
        Ok(value)
    }

    pub(super) fn root_binding(&self) -> Result<String, FitError> {
        self.verify_root()?;
        Ok(self.binding.clone())
    }

    pub(super) fn read_file(
        &self,
        path: &CanonicalPath,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, FitError> {
        self.verify_root()?;
        if maximum_bytes == 0 || maximum_bytes > super::super::model::MAX_FILE_BYTES {
            return Err(error(FitErrorId::ResourceLimit));
        }
        let mut enumeration_budget = EnumerationBudget::new();
        let parts = path.components().collect::<Vec<_>>();
        let mut directory = duplicate(&self.root)?;
        let mut ancestors = Vec::new();
        let mut expected_parent = self.canonical.clone();
        for component in &parts[..parts.len() - 1] {
            let parent_expected = identity(
                &directory
                    .metadata()
                    .map_err(|_| error(FitErrorId::ReadFailed))?,
            );
            match exact_entry(&directory, component, &mut enumeration_budget)? {
                EntryMatch::Absent => {
                    self.revalidate_attachments(&ancestors, &mut enumeration_budget)?;
                    self.verify_root()?;
                    return platform_path::finish_absent(
                        self,
                        &directory,
                        &expected_parent,
                        component,
                        parent_expected,
                        &mut enumeration_budget,
                    );
                }
                EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
                EntryMatch::Exact => {}
            }
            let before =
                stat_at(&directory, component)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
            #[cfg(all(test, target_vendor = "apple"))]
            tests::observe(tests::ReadEvent::BeforeAncestorOpen);
            let next = open_at(
                &directory,
                component,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?
            .ok_or_else(|| error(FitErrorId::StaleBinding))?;
            let opened = next.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
            #[cfg(all(test, target_vendor = "apple"))]
            tests::observe(tests::ReadEvent::AfterAncestorOpen);
            let after =
                stat_at(&directory, component)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
            if !opened.is_dir()
                || before.device != opened.dev()
                || before.inode != opened.ino()
                || before != after
                || exact_entry(&directory, component, &mut enumeration_budget)? != EntryMatch::Exact
            {
                return Err(error(FitErrorId::UnsafeObject));
            }
            ancestors.push((duplicate(&directory)?, (*component).to_string(), before));
            directory = next;
            expected_parent.push(component);
        }
        let name = parts.last().expect("canonical path is nonempty");
        let result = self.read_leaf(&directory, name, maximum_bytes, &mut enumeration_budget)?;
        self.revalidate_attachments(&ancestors, &mut enumeration_budget)?;
        #[cfg(all(test, target_vendor = "apple"))]
        tests::observe(tests::ReadEvent::BeforeFinalRootCheck);
        self.verify_root()?;
        match result {
            platform_path::LeafRead::Absent(parent_expected) => platform_path::finish_absent(
                self,
                &directory,
                &expected_parent,
                name,
                parent_expected,
                &mut enumeration_budget,
            ),
            platform_path::LeafRead::Present {
                bytes,
                file,
                expected,
                parent_expected,
            } => platform_path::finish_present(
                self,
                &directory,
                &expected_parent,
                name,
                bytes,
                file,
                expected,
                parent_expected,
                maximum_bytes,
                &mut enumeration_budget,
            ),
        }
    }

    fn revalidate_attachments(
        &self,
        ancestors: &[(File, String, PathStat)],
        enumeration_budget: &mut EnumerationBudget,
    ) -> Result<(), FitError> {
        for (parent, name, expected) in ancestors.iter().rev() {
            if exact_entry(parent, name, enumeration_budget)? != EntryMatch::Exact
                || stat_at(parent, name)? != Some(*expected)
            {
                return Err(error(FitErrorId::StaleBinding));
            }
            #[cfg(all(test, target_vendor = "apple"))]
            tests::observe(tests::ReadEvent::AfterAttachmentRevalidated);
        }
        Ok(())
    }

    fn read_leaf(
        &self,
        directory: &File,
        name: &str,
        maximum_bytes: usize,
        enumeration_budget: &mut EnumerationBudget,
    ) -> Result<platform_path::LeafRead, FitError> {
        let parent_expected = identity(
            &directory
                .metadata()
                .map_err(|_| error(FitErrorId::ReadFailed))?,
        );
        match exact_entry(directory, name, enumeration_budget)? {
            EntryMatch::Absent => return Ok(platform_path::LeafRead::Absent(parent_expected)),
            EntryMatch::Alias => return Err(error(FitErrorId::UnsafeObject)),
            EntryMatch::Exact => {}
        }
        let path_before =
            stat_at(directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
        if !path_before.regular
            || path_before.links != 1
            || path_before.length > maximum_bytes as u64
        {
            return Err(error(if path_before.length > maximum_bytes as u64 {
                FitErrorId::ResourceLimit
            } else {
                FitErrorId::UnsafeObject
            }));
        }
        let mut file = open_at(
            directory,
            name,
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )?
        .ok_or_else(|| error(FitErrorId::StaleBinding))?;
        let opened = file.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
        if identity(&opened) != path_before {
            return Err(error(FitErrorId::StaleBinding));
        }
        let first = bounded_read(&mut file, maximum_bytes)?;
        #[cfg(all(test, target_vendor = "apple"))]
        tests::observe(tests::ReadEvent::AfterFirstLeafRead);
        file.seek(SeekFrom::Start(0))
            .map_err(|_| error(FitErrorId::ReadFailed))?;
        let second = bounded_read(&mut file, maximum_bytes)?;
        let opened_after = file.metadata().map_err(|_| error(FitErrorId::ReadFailed))?;
        let path_after =
            stat_at(directory, name)?.ok_or_else(|| error(FitErrorId::StaleBinding))?;
        if first != second
            || first.len() as u64 != path_before.length
            || identity(&opened_after) != path_before
            || path_after != path_before
            || exact_entry(directory, name, enumeration_budget)? != EntryMatch::Exact
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        Ok(platform_path::LeafRead::Present {
            bytes: first,
            file,
            expected: path_before,
            parent_expected,
        })
    }

    fn verify_root(&self) -> Result<(), FitError> {
        let path = fs::symlink_metadata(&self.path).map_err(|_| error(FitErrorId::StaleBinding))?;
        let handle = self
            .root
            .metadata()
            .map_err(|_| error(FitErrorId::StaleBinding))?;
        if path.file_type().is_symlink()
            || !path.is_dir()
            || path.dev() != self.device
            || path.ino() != self.inode
            || handle.dev() != self.device
            || handle.ino() != self.inode
            || descriptor_path(&self.root)? != self.canonical
            || fs::canonicalize(&self.path).map_err(|_| error(FitErrorId::StaleBinding))?
                != self.canonical
        {
            return Err(error(FitErrorId::StaleBinding));
        }
        Ok(())
    }
}

fn bounded_read(file: &mut File, maximum: usize) -> Result<Vec<u8>, FitError> {
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

fn identity(metadata: &fs::Metadata) -> PathStat {
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

#[cfg(all(test, target_vendor = "apple"))]
#[path = "unix_tests.rs"]
mod tests;
