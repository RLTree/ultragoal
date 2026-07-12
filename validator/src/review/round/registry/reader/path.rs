use super::Error;
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

const MAX_RELATIVE_PATH_BYTES: usize = 512;
const MAX_PATH_COMPONENT_BYTES: usize = 128;
const MAX_PATH_DEPTH: usize = 32;

#[derive(Clone, Copy, Eq, PartialEq)]
struct FileSnapshot {
    device: u64,
    inode: u64,
    links: u64,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl FileSnapshot {
    fn of(metadata: &std::fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified: (metadata.mtime(), metadata.mtime_nsec()),
            changed: (metadata.ctime(), metadata.ctime_nsec()),
        }
    }
}

pub(super) struct Observation {
    names: Vec<OsString>,
    directories: Vec<(u64, u64)>,
    file: FileSnapshot,
}

impl Observation {
    pub(super) fn capacity(&self) -> usize {
        usize::try_from(self.file.length).unwrap_or(0)
    }

    pub(super) fn revalidate(&self, root: &File, maximum: u64) -> Result<(), Error> {
        let (file, current) =
            open_observed(root, self.names.clone(), maximum).map_err(|_| Error::Changed)?;
        drop(file);
        if self.directories != current.directories || self.file != current.file {
            return Err(Error::Changed);
        }
        Ok(())
    }
}

pub(super) fn normal_names(relative: &str) -> Result<Vec<OsString>, Error> {
    if relative.is_empty() || relative.len() > MAX_RELATIVE_PATH_BYTES {
        return Err(Error::Invalid);
    }
    let names = Path::new(relative)
        .components()
        .map(|part| match part {
            Component::Normal(name)
                if !name.as_bytes().is_empty()
                    && name.as_bytes().len() <= MAX_PATH_COMPONENT_BYTES =>
            {
                Ok(name.to_os_string())
            }
            _ => Err(Error::Invalid),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if names.is_empty() || names.len() > MAX_PATH_DEPTH {
        return Err(Error::Invalid);
    }
    Ok(names)
}

pub(super) fn open_observed(
    root: &File,
    names: Vec<OsString>,
    maximum: u64,
) -> Result<(File, Observation), Error> {
    let (leaf, parents) = names.split_last().ok_or(Error::Invalid)?;
    let mut directory = root.try_clone().map_err(|_| Error::Invalid)?;
    let mut directories = Vec::new();
    for name in parents {
        directory = open_at(
            &directory,
            name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )?;
        directories.push(directory_identity(&directory)?);
    }
    let file = open_at(
        &directory,
        leaf,
        libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
    )?;
    let snapshot = regular_snapshot(&file, maximum)?;
    Ok((
        file,
        Observation {
            names,
            directories,
            file: snapshot,
        },
    ))
}

fn open_at(parent: &File, name: &OsStr, flags: i32) -> Result<File, Error> {
    let name = CString::new(name.as_bytes()).map_err(|_| Error::Invalid)?;
    let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(Error::Invalid);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub(super) fn directory_identity(file: &File) -> Result<(u64, u64), Error> {
    let metadata = file.metadata().map_err(|_| Error::Invalid)?;
    if !metadata.is_dir() {
        return Err(Error::Invalid);
    }
    Ok((metadata.dev(), metadata.ino()))
}

fn regular_snapshot(file: &File, maximum: u64) -> Result<FileSnapshot, Error> {
    let metadata = file.metadata().map_err(|_| Error::Invalid)?;
    if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > maximum {
        return Err(Error::Invalid);
    }
    Ok(FileSnapshot::of(&metadata))
}
