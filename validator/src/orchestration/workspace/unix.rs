use super::super::{
    CanonicalPath, OrchestrationError, RootIntegrationIntent, RootIntegrationObservation,
};
use super::sys::{EntryMatch, duplicate, exact_entry, open_at, stat_at};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

pub(super) struct Workspace {
    path: PathBuf,
    canonical: PathBuf,
    root: File,
    device: u64,
    inode: u64,
}

impl Workspace {
    pub(super) fn open(path: &Path) -> Result<Self, OrchestrationError> {
        let path_metadata =
            fs::symlink_metadata(path).map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        if path_metadata.file_type().is_symlink() || !path_metadata.is_dir() {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        let mut options = OpenOptions::new();
        options.read(true).custom_flags(
            libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        );
        let root = options
            .open(path)
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        let metadata = root
            .metadata()
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        if !metadata.is_dir()
            || metadata.dev() != path_metadata.dev()
            || metadata.ino() != path_metadata.ino()
        {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        let value = Self {
            path: path.to_path_buf(),
            canonical: fs::canonicalize(path)
                .map_err(|_| OrchestrationError::IntegrationAmbiguous)?,
            root,
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        value.verify_root()?;
        Ok(value)
    }

    pub(super) fn observe(
        &self,
        intent: &RootIntegrationIntent,
    ) -> Result<RootIntegrationObservation, OrchestrationError> {
        intent.validate()?;
        self.verify_root()?;
        let mut observed = BTreeMap::new();
        let mut evidence = Sha256::new();
        evidence.update(self.device.to_le_bytes());
        evidence.update(self.inode.to_le_bytes());
        let mut total = 0_u64;
        for raw in intent.expected_digests.keys() {
            let path = CanonicalPath::parse(raw)?;
            let value = self.read_path(&path)?;
            evidence.update((raw.len() as u64).to_le_bytes());
            evidence.update(raw.as_bytes());
            match value {
                Some((digest, length)) => {
                    total = total
                        .checked_add(length)
                        .ok_or(OrchestrationError::ResourceLimit)?;
                    if total > MAX_TOTAL_BYTES {
                        return Err(OrchestrationError::ResourceLimit);
                    }
                    evidence.update([1]);
                    evidence.update(length.to_le_bytes());
                    evidence.update(digest.as_bytes());
                    observed.insert(raw.clone(), Some(digest));
                }
                None => {
                    evidence.update([0]);
                    observed.insert(raw.clone(), None);
                }
            }
        }
        self.verify_root()?;
        intent.observe(observed, &format!("sha256:{:x}", evidence.finalize()))
    }

    fn read_path(&self, path: &CanonicalPath) -> Result<Option<(String, u64)>, OrchestrationError> {
        let mut directory = duplicate(&self.root)?;
        let parts: Vec<_> = path.as_str().split('/').collect();
        for component in &parts[..parts.len() - 1] {
            match exact_entry(&directory, component)? {
                EntryMatch::Absent => return Ok(None),
                EntryMatch::Alias => return Err(OrchestrationError::IntegrationAmbiguous),
                EntryMatch::Exact => {}
            }
            let before =
                stat_at(&directory, component)?.ok_or(OrchestrationError::IntegrationAmbiguous)?;
            let next = open_at(
                &directory,
                component,
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )?
            .ok_or(OrchestrationError::IntegrationAmbiguous)?;
            let opened = next
                .metadata()
                .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
            let after =
                stat_at(&directory, component)?.ok_or(OrchestrationError::IntegrationAmbiguous)?;
            if !opened.is_dir()
                || before.device != opened.dev()
                || before.inode != opened.ino()
                || before.device != after.device
                || before.inode != after.inode
                || exact_entry(&directory, component)? != EntryMatch::Exact
            {
                return Err(OrchestrationError::IntegrationAmbiguous);
            }
            directory = next;
        }
        let name = parts.last().expect("canonical path has a component");
        match exact_entry(&directory, name)? {
            EntryMatch::Absent => return Ok(None),
            EntryMatch::Alias => return Err(OrchestrationError::IntegrationAmbiguous),
            EntryMatch::Exact => {}
        }
        let Some(mut file) = open_at(
            &directory,
            name,
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )?
        else {
            return Err(OrchestrationError::IntegrationAmbiguous);
        };
        let before = file
            .metadata()
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        if !before.is_file() || before.nlink() != 1 || before.len() > MAX_FILE_BYTES {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        let mut bytes = Vec::with_capacity(before.len() as usize);
        Read::by_ref(&mut file)
            .take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        let after = file
            .metadata()
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        let path_stat =
            stat_at(&directory, name)?.ok_or(OrchestrationError::IntegrationAmbiguous)?;
        if bytes.len() as u64 != before.len()
            || identity(&before) != identity(&after)
            || before.nlink() != after.nlink()
            || path_stat.device != before.dev()
            || path_stat.inode != before.ino()
            || path_stat.links != 1
            || !path_stat.regular
            || exact_entry(&directory, name)? != EntryMatch::Exact
        {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        Ok(Some((
            format!("sha256:{:x}", Sha256::digest(&bytes)),
            bytes.len() as u64,
        )))
    }

    fn verify_root(&self) -> Result<(), OrchestrationError> {
        let path = fs::symlink_metadata(&self.path)
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        let handle = self
            .root
            .metadata()
            .map_err(|_| OrchestrationError::IntegrationAmbiguous)?;
        if path.file_type().is_symlink()
            || !path.is_dir()
            || path.dev() != self.device
            || path.ino() != self.inode
            || handle.dev() != self.device
            || handle.ino() != self.inode
            || fs::canonicalize(&self.path).map_err(|_| OrchestrationError::IntegrationAmbiguous)?
                != self.canonical
        {
            return Err(OrchestrationError::IntegrationAmbiguous);
        }
        Ok(())
    }
}

fn identity(metadata: &fs::Metadata) -> (u64, u64, u64, i64, i64) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
    )
}
