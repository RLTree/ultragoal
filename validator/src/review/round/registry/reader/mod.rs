mod json;
#[cfg(unix)]
mod path;

#[cfg(unix)]
use path::{Observation, directory_identity, normal_names, open_observed};
use serde_json::Value;
#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

pub(super) const MAX_BYTES: u64 = 4 * 1024 * 1024;
pub(super) struct Artifact {
    pub(super) bytes: Vec<u8>,
    pub(super) value: Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Error {
    Invalid,
    Changed,
    Malformed,
}

#[cfg(all(test, unix))]
thread_local! {
    static AFTER_READ: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
    static BEFORE_FINAL_REVALIDATE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
}

#[cfg(all(test, unix))]
pub(crate) fn set_after_read_hook(hook: impl FnOnce() + 'static) {
    AFTER_READ.with(|slot| assert!(slot.borrow_mut().replace(Box::new(hook)).is_none()));
}

#[cfg(all(test, unix))]
pub(crate) fn set_before_final_revalidate_hook(hook: impl FnOnce() + 'static) {
    BEFORE_FINAL_REVALIDATE.with(|slot| {
        assert!(slot.borrow_mut().replace(Box::new(hook)).is_none());
    });
}

#[cfg(all(test, unix))]
fn run_after_read_hook() {
    AFTER_READ.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(all(test, unix))]
fn run_before_final_revalidate_hook() {
    BEFORE_FINAL_REVALIDATE.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(all(test, unix)))]
fn run_after_read_hook() {}

#[cfg(not(all(test, unix)))]
fn run_before_final_revalidate_hook() {}

#[cfg(unix)]
pub(super) struct Session {
    root: PathBuf,
    root_file: File,
    root_identity: (u64, u64),
    observations: Vec<Observation>,
}

#[cfg(not(unix))]
pub(super) struct Session;

#[cfg(unix)]
impl Session {
    pub(super) fn new(root: &Path) -> Result<Self, Error> {
        if root.canonicalize().map_err(|_| Error::Invalid)? != root {
            return Err(Error::Invalid);
        }
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW);
        let root_file = options.open(root).map_err(|_| Error::Invalid)?;
        let root_identity = directory_identity(&root_file)?;
        Ok(Self {
            root: root.to_path_buf(),
            root_file,
            root_identity,
            observations: Vec::new(),
        })
    }

    pub(super) fn read_json(&mut self, relative: &str) -> Result<Artifact, Error> {
        let bytes = self.read_bytes(relative, MAX_BYTES)?;
        let value = json::parse_unique_json(&bytes)?;
        Ok(Artifact { bytes, value })
    }

    pub(super) fn read_bytes(&mut self, relative: &str, maximum: u64) -> Result<Vec<u8>, Error> {
        if maximum > MAX_BYTES {
            return Err(Error::Invalid);
        }
        let names = normal_names(relative)?;
        let (mut file, observation) = open_observed(&self.root_file, names, maximum)?;
        let mut bytes = Vec::with_capacity(observation.capacity());
        file.by_ref()
            .take(maximum + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Invalid)?;
        if bytes.len() as u64 > maximum {
            return Err(Error::Invalid);
        }
        run_after_read_hook();
        observation.revalidate(&self.root_file, maximum)?;
        self.observations.push(observation);
        Ok(bytes)
    }

    pub(super) fn revalidate(&self) -> Result<(), Error> {
        run_before_final_revalidate_hook();
        self.revalidate_root()?;
        for observation in &self.observations {
            observation.revalidate(&self.root_file, MAX_BYTES)?;
        }
        self.revalidate_root()
    }

    fn revalidate_root(&self) -> Result<(), Error> {
        let metadata = std::fs::symlink_metadata(&self.root).map_err(|_| Error::Changed)?;
        if metadata.file_type().is_symlink()
            || !metadata.is_dir()
            || (metadata.dev(), metadata.ino()) != self.root_identity
        {
            return Err(Error::Changed);
        }
        Ok(())
    }
}

#[cfg(not(unix))]
impl Session {
    pub(super) fn new(_: &Path) -> Result<Self, Error> {
        Err(Error::Invalid)
    }
    pub(super) fn read_json(&mut self, _: &str) -> Result<Artifact, Error> {
        Err(Error::Invalid)
    }
    pub(super) fn read_bytes(&mut self, _: &str, _: u64) -> Result<Vec<u8>, Error> {
        Err(Error::Invalid)
    }
    pub(super) fn revalidate(&self) -> Result<(), Error> {
        Err(Error::Invalid)
    }
}
