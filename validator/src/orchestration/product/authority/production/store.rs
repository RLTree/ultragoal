use super::ProductError;
use getrandom::fill;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

include!("store_file.rs");

const ACTOR_NAME: &str = "root-actor";
const KEY_NAME: &str = "authority-key";
const LOCK_NAME: &str = "replay-lock";
const LEDGER_NAME: &str = "replay-ledger.jsonl";
const MAX_LEDGER_BYTES: u64 = 2 * 1024 * 1024;

pub(super) struct Store {
    root: PathBuf,
    directory: File,
    root_device: u64,
    root_inode: u64,
    lock_device: u64,
    lock_inode: u64,
    ledger_device: u64,
    ledger_inode: u64,
    actor_record: Vec<u8>,
    key: [u8; 32],
}

pub(super) struct ProcessLock(File);

impl Store {
    pub(super) fn open_or_initialize(
        root: &Path,
        actor: &str,
    ) -> Result<(Self, [u8; 32]), ProductError> {
        let directory = open_directory(root)?;
        let lock = open_file_at(&directory, LOCK_NAME, libc::O_RDWR | libc::O_CREAT, 256)?;
        let lock_identity = file_identity(&lock)?;
        let _guard = ProcessLock::acquire(lock)?;
        let present = [
            entry_exists(&directory, ACTOR_NAME)?,
            entry_exists(&directory, KEY_NAME)?,
            entry_exists(&directory, LEDGER_NAME)?,
        ];
        if present.iter().any(|value| *value) && !present.iter().all(|value| *value) {
            recover_partial_initialization(&directory, &present)?;
        }
        if !entry_exists(&directory, ACTOR_NAME)? {
            let mut key = [0_u8; 32];
            fill(&mut key).map_err(|_| ProductError::AuthorityStoreInvalid)?;
            create_file_at(&directory, LEDGER_NAME, b"")?;
            let ledger = open_file_at(&directory, LEDGER_NAME, libc::O_RDONLY, 0)?;
            let ledger_identity = file_identity(&ledger)?;
            create_file_at(&directory, KEY_NAME, &key)?;
            create_file_at(
                &directory,
                ACTOR_NAME,
                actor_record(actor, lock_identity, ledger_identity).as_bytes(),
            )?;
            directory
                .sync_all()
                .map_err(|_| ProductError::AuthorityStoreInvalid)?;
            key.fill(0);
        }
        Self::open_existing_locked(root, directory, actor, lock_identity)
    }

    pub(super) fn open_existing(
        root: &Path,
        actor: &str,
    ) -> Result<(Self, [u8; 32]), ProductError> {
        let directory = open_directory(root)?;
        let lock = open_file_at(&directory, LOCK_NAME, libc::O_RDWR, 256)?;
        let lock_identity = file_identity(&lock)?;
        let _guard = ProcessLock::acquire(lock)?;
        Self::open_existing_locked(root, directory, actor, lock_identity)
    }

    fn open_existing_locked(
        root: &Path,
        directory: File,
        actor: &str,
        lock_identity: (u64, u64),
    ) -> Result<(Self, [u8; 32]), ProductError> {
        verify_directory_path(root, &directory)?;
        for name in [ACTOR_NAME, KEY_NAME, LOCK_NAME, LEDGER_NAME] {
            let max = if name == LEDGER_NAME {
                MAX_LEDGER_BYTES
            } else {
                256
            };
            open_file_at(&directory, name, libc::O_RDONLY, max)?;
        }
        let ledger = open_file_at(&directory, LEDGER_NAME, libc::O_RDONLY, MAX_LEDGER_BYTES)?;
        let ledger_identity = file_identity(&ledger)?;
        let actor_record = actor_record(actor, lock_identity, ledger_identity).into_bytes();
        if read_bounded_at(&directory, ACTOR_NAME, 256)? != actor_record {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        let key: [u8; 32] = read_bounded_at(&directory, KEY_NAME, 32)?
            .try_into()
            .map_err(|_| ProductError::AuthorityStoreInvalid)?;
        let metadata = directory
            .metadata()
            .map_err(|_| ProductError::AuthorityStoreInvalid)?;
        let store = Self {
            root: root.to_path_buf(),
            directory,
            root_device: metadata.dev(),
            root_inode: metadata.ino(),
            lock_device: lock_identity.0,
            lock_inode: lock_identity.1,
            ledger_device: ledger_identity.0,
            ledger_inode: ledger_identity.1,
            actor_record,
            key,
        };
        store.verify()?;
        Ok((store, key))
    }

    pub(super) const fn key(&self) -> &[u8; 32] {
        &self.key
    }

    pub(super) fn lock(&self) -> Result<ProcessLock, ProductError> {
        self.verify()?;
        let lock = open_file_at(&self.directory, LOCK_NAME, libc::O_RDWR, 256)?;
        self.verify_lock(&lock)?;
        ProcessLock::acquire(lock)
    }

    pub(super) fn read_ledger(&self) -> Result<Vec<u8>, ProductError> {
        let file = open_file_at(
            &self.directory,
            LEDGER_NAME,
            libc::O_RDONLY,
            MAX_LEDGER_BYTES,
        )?;
        self.verify_ledger(&file)?;
        read_bounded_file(file, MAX_LEDGER_BYTES)
    }

    pub(super) fn ledger_stamp(&self) -> Result<(u64, i64, i64), ProductError> {
        let file = open_file_at(
            &self.directory,
            LEDGER_NAME,
            libc::O_RDONLY,
            MAX_LEDGER_BYTES,
        )?;
        self.verify_ledger(&file)?;
        let metadata = file
            .metadata()
            .map_err(|_| ProductError::AuthorityStoreInvalid)?;
        Ok((metadata.len(), metadata.ctime(), metadata.ctime_nsec()))
    }

    pub(super) fn append_record(&self, bytes: &[u8]) -> Result<(), ProductError> {
        if bytes.is_empty() || bytes.len() > 16 * 1024 || bytes.contains(&b'\n') {
            return Err(ProductError::AuthorityStoreInvalid);
        }
        let file = open_file_at(
            &self.directory,
            LEDGER_NAME,
            libc::O_WRONLY | libc::O_APPEND,
            MAX_LEDGER_BYTES,
        )?;
        self.verify_ledger(&file)?;
        append_frame(&file, bytes)?;
        validate_opened_file(&file, MAX_LEDGER_BYTES)?;
        self.verify()
    }
}

include!("store_initialization.rs");
include!("store_verification.rs");

fn validate_root(root: &Path) -> Result<(), ProductError> {
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    let metadata = fs::symlink_metadata(root).map_err(|_| ProductError::AuthorityStoreInvalid)?;
    let effective_uid = effective_uid();
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || metadata.uid() != effective_uid
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(ProductError::AuthorityStoreInvalid);
    }
    Ok(())
}
